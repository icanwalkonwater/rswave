use crate::{audio::AudioProcessor, Opt};
use anyhow::{anyhow, ensure, Result};
use byteorder::{ReadBytesExt};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream,
};
use parking_lot::Mutex;
use ringbuf::{Consumer, RingBuffer};
use rpi_led_common::MAGIC;
use std::{
    io::{stdout, Stdout},
    net::TcpStream,
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

struct AudioHolder {
    device: cpal::Device,
    stream: Option<Stream>,
    consumer: Option<Consumer<f64>>,
    processor: AudioProcessor,
}

pub struct App {
    opt: Opt,
    socket: Option<TcpStream>,
    audio: AudioHolder,
    tui: Terminal<CrosstermBackend<Stdout>>,

    run_time: Duration,
}

impl App {
    pub fn new() -> Result<Arc<Mutex<Self>>> {
        let opt: Opt = Opt::from_args();

        // Create socket if specified
        let socket = if let Some(address) = opt.address.as_ref() {
            Some(TcpStream::connect(address)?)
        } else {
            eprintln!("No address provided ! No connection will be made");
            None
        };

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

        let mut tui = Terminal::new(CrosstermBackend::new(stdout()))?;
        tui.clear()?;

        Ok(Arc::new(Mutex::new(Self {
            opt,
            socket,
            audio: AudioHolder {
                device: audio_device,
                stream: None,
                consumer: None,
                processor: Default::default(),
            },
            tui,
            run_time: Duration::from_millis(0),
        })))
    }

    pub fn init_network(&mut self) -> Result<()> {
        if let None = self.socket {
            return Ok(());
        }
        let socket = self.socket.as_mut().unwrap();

        // Read hello from server
        let magic = socket.read_u8()?;
        ensure!(magic == MAGIC, "Magic number is wrong");

        // Write mode
        // socket.write_u8(self.mode.int_value())?;

        Ok(())
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

    pub fn run_once(&mut self) -> Result<()> {
        let start = Instant::now();

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

        // Time
        self.run_time = Instant::now().duration_since(start);
        Ok(())
    }

    pub fn draw(&mut self) {
        let raw_data = self
            .audio
            .processor
            .input()
            .chunks_exact(2)
            .enumerate()
            .map(|(i, slice)| (i as f64, slice[0].max(slice[1])))
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

        let max_data = self.audio.processor.peak_input() * 1.5;
        let max_fft = self.audio.processor.peak_output() * 1.2;

        let mut bar_data = Vec::with_capacity(6);
        for (bar, range) in self.audio.processor.bars_data() {
            bar_data.push((range.start as f64, *bar));
            bar_data.push((range.end as f64 - 1.0, *bar));
        }

        let deltas = self
            .audio
            .processor
            .deltas()
            .iter()
            .fold((0.0, 0.0, 0.0), |acc, bar| {
                (acc.0 + bar[0], acc.1 + bar[1], acc.2 + bar[2])
            });
        let peak_delta = self.audio.processor.peak_delta() * 2.0;

        let run_time_micros = self.run_time.as_micros();

        self.tui
            .draw(|frame| {
                let main_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                    .split(frame.size());

                let graph_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
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
                                .title(" PCM Data - Max L/R ")
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

                    let bar_dataset = Dataset::default()
                        .marker(Marker::Braille)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(Color::Yellow))
                        .data(&bar_data);

                    Chart::new(vec![fft_dataset, bar_dataset])
                        .block(
                            Block::default()
                                .title(" FFT Data Magnitude - log2 ")
                                .borders(Borders::ALL),
                        )
                        .x_axis(Axis::default().bounds([0.0, fft_data.len() as f64]))
                        .y_axis(Axis::default().bounds([0.0, max_fft]))
                };

                let output_data_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Length(3),
                            Constraint::Length(2),
                            Constraint::Length(3),
                            Constraint::Min(1),
                        ]
                        .as_ref(),
                    )
                    .split(main_layout[1]);

                let status = {
                    let text = vec![Spans::from(vec![
                        Span::styled(
                            " Process time: ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{}us", run_time_micros)),
                    ])];

                    Paragraph::new(text)
                        .block(Block::default().title(" Status ").borders(Borders::ALL))
                        .alignment(Alignment::Left)
                };

                let delta_bar_0 = {
                    Gauge::default()
                        .block(
                            Block::default()
                                .title(" Smoothed Deltas ")
                                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT),
                        )
                        .gauge_style(Style::default().fg(Color::White))
                        .label("Bass")
                        .ratio((deltas.0 / peak_delta).min(1.0))
                };

                let delta_bar_1 = {
                    Gauge::default()
                        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
                        .gauge_style(Style::default().fg(Color::White))
                        .label("Mid")
                        .ratio((deltas.1 / peak_delta).min(1.0))
                };

                let delta_bar_2 = {
                    Gauge::default()
                        .block(
                            Block::default()
                                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM),
                        )
                        .gauge_style(Style::default().fg(Color::White))
                        .label("Treble")
                        .ratio((deltas.2 / peak_delta).min(1.0))
                };

                frame.render_widget(raw_graph, graph_layout[0]);
                frame.render_widget(fft_graph, graph_layout[1]);
                frame.render_widget(status, output_data_layout[0]);
                frame.render_widget(delta_bar_0, output_data_layout[1]);
                frame.render_widget(delta_bar_1, output_data_layout[2]);
                frame.render_widget(delta_bar_2, output_data_layout[3]);
            })
            .unwrap();
    }
}
