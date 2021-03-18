use crate::{
    audio::{AudioProcessor, COMPRESSION_CONST},
    net::NetHandler,
    spotify::SpotifyTracker,
    Opt,
};
use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream,
};
use parking_lot::Mutex;
use ringbuf::{Consumer, RingBuffer};
use std::{
    cmp::Ordering,
    io::{stdout, Stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Span, Spans},
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, Paragraph},
    Terminal,
};

pub(crate) struct AudioHolder {
    device: cpal::Device,
    stream: Option<Stream>,
    consumer: Option<Consumer<f64>>,
    pub(crate) processor: AudioProcessor,
}

pub struct App {
    pub(crate) opt: Opt,
    pub(crate) audio: AudioHolder,
    tui: Option<Terminal<CrosstermBackend<Stdout>>>,

    pub(crate) spotify: Option<SpotifyTracker>,
    pub(crate) net: Option<NetHandler>,

    run_time: Duration,
    draw_time: Duration,
    last_run_end: Instant,
    spare_time: Duration,
}

impl App {
    pub async fn new() -> Result<Arc<Mutex<Self>>> {
        let opt: Opt = Opt::from_args();

        // Check options
        match (opt.spotify_id.as_ref(), opt.spotify_secret.as_ref()) {
            (Some(_), Some(_)) | (None, None) => {}
            _ => {
                return Err(anyhow!(
                    "You must provide --spotify-id and --spotify-secret or neither !"
                ))
            }
        }

        // Init audio
        let audio_device = {
            let host = cpal::default_host();
            if let Some(hint) = opt.device_hint.as_ref() {
                host.input_devices()?
                    .find(|device| device.name().map(|n| n.contains(hint)).unwrap_or(false))
                    .ok_or(anyhow!("Can't find a device satisfying the hint"))?
            } else {
                host.default_input_device()
                    .ok_or(anyhow!("No default device found"))?
            }
        };

        // Init spotify
        let spotify = if let (Some(id), Some(secret)) =
            (opt.spotify_id.as_ref(), opt.spotify_secret.as_ref())
        {
            Some(SpotifyTracker::new(id, secret).await?)
        } else {
            None
        };

        // Init net
        let net = if let Some(addr) = opt.address.as_ref() {
            let mut net = NetHandler::new(addr)?;
            net.handshake()?;
            Some(net)
        } else {
            None
        };

        // Init TUI
        let tui = if opt.no_tui {
            None
        } else {
            let mut tui = Terminal::new(CrosstermBackend::new(stdout()))?;
            // Clear terminal just before creating the app
            tui.clear()?;
            Some(tui)
        };

        Ok(Arc::new(Mutex::new(Self {
            opt,
            audio: AudioHolder {
                device: audio_device,
                stream: None,
                consumer: None,
                processor: Default::default(),
            },
            tui,
            spotify,
            net,
            run_time: Duration::from_millis(0),
            draw_time: Duration::from_millis(0),
            last_run_end: Instant::now(),
            spare_time: Duration::from_millis(0),
        })))
    }

    pub fn recreate_audio_stream(&mut self) -> Result<()> {
        // Drop previous stuff
        {
            self.audio.stream.take();
            self.audio.consumer.take();
        }

        let config = self.audio.device.default_input_config()?;
        assert_eq!(
            config.sample_rate(),
            SampleRate(44100),
            "Only 44100Hz sample rate supported !"
        );
        assert_eq!(config.channels(), 2, "Only stereo is supported !");

        // Ring buffer 4 times as large as the sample size, so we can store a total of 2 frames of 2 channels
        let (mut prod, cons) = RingBuffer::new(self.audio.processor.sample_size() * 4).split();

        let reader = match config.sample_format() {
            SampleFormat::I16 => self.audio.device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    prod.push_iter(&mut data.iter().copied().map(|sample| sample as f64));
                },
                |e| eprintln!("CPAL Error: {:?}", e),
            ),
            SampleFormat::U16 => self.audio.device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    prod.push_iter(
                        &mut data
                            .iter()
                            .copied()
                            .map(|sample| sample as f64 / u16::max_value() as f64 - 0.5),
                    );
                },
                |e| eprintln!("CPAL Error: {:?}", e),
            ),
            SampleFormat::F32 => self.audio.device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    prod.push_iter(&mut data.iter().copied().map(|sample| sample as f64));
                },
                |e| eprintln!("CPAL Error: {:?}", e),
            ),
        }?;

        self.audio.stream = Some(reader);
        self.audio.consumer = Some(cons);

        Ok(())
    }

    pub fn start_recording(&mut self) -> Result<()> {
        if let None = self.audio.stream {
            self.recreate_audio_stream()?;
        }

        self.audio.stream.as_ref().unwrap().play()?;
        Ok(())
    }
}

impl App {
    pub fn can_run(&self) -> bool {
        self.audio.consumer.as_ref().map_or(false, |cons| {
            cons.len() > self.audio.processor.sample_size() * 2
        })
    }

    pub async fn run_once(&mut self) -> Result<()> {
        let start = Instant::now();
        self.spare_time = start.duration_since(self.last_run_end);

        if let None = self.audio.stream {
            self.recreate_audio_stream()?;
        }

        // Read audio
        assert!(self.can_run());
        self.audio
            .consumer
            .as_mut()
            .unwrap()
            .pop_slice(self.audio.processor.input());
        // Process it
        self.audio.processor.process();
        // That was easy

        // Refresh spotify
        if let Some(spotify) = self.spotify.as_mut() {
            spotify.refresh_current_track().await;
            spotify.advance_beat();
        }

        // Send to remote and acknowledge
        if let Some(net) = self.net.as_mut() {
            net.send_current_data(
                &self.audio.processor,
                self.spotify.as_ref(),
                self.opt.no_ack,
            )?;
        }

        // Time
        self.run_time = Instant::now().duration_since(start);
        self.last_run_end = Instant::now();
        Ok(())
    }

    pub fn draw(&mut self) {
        if let None = self.tui {
            return;
        }
        let tui = self.tui.as_mut().unwrap();

        let start = Instant::now();

        // Curve data

        let raw_data = self
            .audio
            .processor
            .input()
            .chunks_exact(2)
            .enumerate()
            .map(|(i, slice)| (i as f64, (slice[0] + slice[1]) / 2.0))
            .collect::<Vec<_>>();

        let fft_data = self
            .audio
            .processor
            .output()
            .iter()
            .copied()
            .enumerate()
            .map(|(i, sample)| (i as f64, sample))
            .collect::<Vec<_>>();

        let last_novelty = self.audio.processor.novelty();
        let novelty_data = self
            .audio
            .processor
            .novelty_curve()
            .enumerate()
            .map(|(i, val)| (i as f64, val))
            .collect::<Vec<(f64, f64)>>();

        // Some max for display

        let max_data = self.audio.processor.peak_input() * 1.1;
        let max_fft = self.audio.processor.peak_output() * 1.2;
        let max_novelty = self
            .audio
            .processor
            .novelty_curve()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0.0);

        let run_time_micros = self.run_time.as_micros();
        let draw_time_micros = self.draw_time.as_micros();
        let spare_time_millis = self.spare_time.as_millis();

        // Spotify info
        let (spotify_online, current_track, tempo, is_beat) =
            if let Some(spotify) = self.spotify.as_ref() {
                (
                    true,
                    spotify.current_track(),
                    spotify.tempo(),
                    spotify.is_beat(),
                )
            } else {
                (false, None, f32::NAN, false)
            };

        tui.draw(|frame| {
            let main_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                .split(frame.size());

            let graph_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(1, 3),
                    ]
                    .as_ref(),
                )
                .split(main_layout[0]);

            let raw_graph = {
                let raw_dataset = Dataset::default()
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(Color::LightGreen))
                    .data(&raw_data);

                Chart::new(vec![raw_dataset])
                    .block(
                        Block::default()
                            .title(format!(" PCM Data - Avg L/R - {} samples ", raw_data.len()))
                            .borders(Borders::ALL),
                    )
                    .x_axis(Axis::default().bounds([0.0, raw_data.len() as f64]))
                    .y_axis(Axis::default().bounds([-max_data, max_data]))
            };

            let fft_graph = {
                let fft_dataset = Dataset::default()
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(Color::LightBlue))
                    .data(&fft_data);

                Chart::new(vec![fft_dataset])
                    .block(
                        Block::default()
                            .title(format!(
                                " FFT Data Magnitude - Compression: {} - {} samples ",
                                COMPRESSION_CONST,
                                fft_data.len()
                            ))
                            .borders(Borders::ALL),
                    )
                    .x_axis(Axis::default().bounds([0.0, fft_data.len() as f64]))
                    .y_axis(Axis::default().bounds([0.0, max_fft]))
            };

            let novelty_graph = {
                let novelty_dataset = Dataset::default()
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(Color::Yellow))
                    .data(&novelty_data);

                Chart::new(vec![novelty_dataset])
                    .block(
                        Block::default()
                            .title(format!(
                                " Novelty Curve - Max: {:.2} - Current: {:.2} ",
                                max_novelty, last_novelty
                            ))
                            .borders(Borders::ALL),
                    )
                    .x_axis(Axis::default().bounds([0.0, novelty_data.len() as f64]))
                    .y_axis(Axis::default().bounds([0.0, max_novelty * 1.1]))
            };

            let output_data_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Length(4),
                        Constraint::Min(1),
                    ]
                    .as_ref(),
                )
                .split(main_layout[1]);

            let bold = Style::default().add_modifier(Modifier::BOLD);

            let status = {
                let text = vec![Spans::from(vec![
                    Span::styled(" Process time: ", bold),
                    Span::raw(format!("{:3}us", run_time_micros)),
                    Span::styled(" | Draw time: ", bold),
                    Span::raw(format!("{:5}us", draw_time_micros)),
                    Span::styled(" | Spare time: ", bold),
                    if spare_time_millis <= 0 {
                        Span::styled(
                            format!("{:3}ms", spare_time_millis),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw(format!("{:3}ms", spare_time_millis))
                    },
                ])];

                Paragraph::new(text)
                    .block(Block::default().title(" Status ").borders(Borders::ALL))
                    .alignment(Alignment::Left)
            };

            let novelty_bar = {
                Gauge::default()
                    .block(Block::default().title(" Novelty ").borders(Borders::ALL))
                    .gauge_style(Style::default().fg(Color::Yellow))
                    .ratio((last_novelty / max_novelty).min(1.0))
            };

            let spotify_status_text = if let Some((playing, progress)) = current_track {
                let full_track = playing.item.as_ref().unwrap();
                let duration = full_track.duration_ms;
                vec![
                    Spans::from(vec![
                        Span::styled(" Status: ", bold),
                        Span::styled(
                            "Online",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Spans::from(vec![
                        Span::styled(" Current track: ", bold),
                        Span::raw(format!(
                            "{} - {}",
                            &full_track.name, &full_track.artists[0].name
                        )),
                    ]),
                    Spans::from(vec![
                        Span::styled(" Current track ID: ", bold),
                        Span::raw(
                            full_track
                                .id
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or("Unknown ID"),
                        ),
                    ]),
                    Spans::from(vec![
                        Span::styled(" Time: ", bold),
                        Span::raw(format!(
                            "{}:{:02} / {}:{:02}",
                            progress / 60_000,
                            progress / 1000 % 60,
                            duration / 60000,
                            duration / 1000 % 60
                        )),
                    ]),
                    Spans::from(vec![
                        Span::styled(" Tempo: ", bold),
                        Span::raw(format!("{:.2}", tempo)),
                    ]),
                    Spans::from(vec![
                        Span::styled(" New Beat: ", bold),
                        if is_beat {
                            Span::styled(
                                "TRUE ",
                                Style::default()
                                    .fg(Color::White)
                                    .bg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::styled("False", Style::default().fg(Color::Red))
                        },
                    ]),
                ]
            } else {
                vec![
                    Spans::from(vec![
                        Span::styled(" Status: ", bold),
                        if spotify_online {
                            Span::styled(
                                "Online",
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::styled(
                                "Offline",
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            )
                        },
                    ]),
                    Spans::from(vec![Span::styled(" No track currently playing !", bold)]),
                ]
            };

            let spotify_status_widget = Paragraph::new(spotify_status_text)
                .block(Block::default().title(" Spotify ").borders(Borders::ALL));

            frame.render_widget(raw_graph, graph_layout[0]);
            frame.render_widget(fft_graph, graph_layout[1]);
            frame.render_widget(novelty_graph, graph_layout[2]);
            frame.render_widget(status, output_data_layout[0]);
            frame.render_widget(novelty_bar, output_data_layout[1]);
            frame.render_widget(spotify_status_widget, output_data_layout[2]);
        })
        .unwrap();

        self.draw_time = Instant::now().duration_since(start);
        self.last_run_end = Instant::now();
    }

    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(audio) = self.audio.stream.as_ref() {
            audio.pause()?;
        }

        if let Some(net) = self.net.as_mut() {
            net.stop(false)?;
        }

        Ok(())
    }
}
