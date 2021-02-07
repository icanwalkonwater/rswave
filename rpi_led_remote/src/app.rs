use crate::Opt;
use anyhow::{anyhow, bail, ensure, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Stream,
};
use parking_lot::Mutex;
use ringbuf::{Consumer, RingBuffer};
use rpi_led_common::{IntEnum, LedMode, MAGIC};
use std::{
    io::{stdout, Stdout},
    net::TcpStream,
    sync::Arc,
};
use structopt::StructOpt;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols::Marker,
    widgets::{Axis, BarChart, Block, Borders, Chart, Dataset, GraphType},
    Terminal,
};

pub struct App {
    opt: Opt,
    mode: LedMode,
    socket: Option<TcpStream>,
    audio_device: cpal::Device,
    tui: Terminal<CrosstermBackend<Stdout>>,
}

impl App {
    pub fn new() -> Result<Arc<Mutex<Self>>> {
        let opt: Opt = Opt::from_args();

        // Choose mode
        let mode = match (opt.only_color, opt.only_intensity) {
            (true, false) => LedMode::OnlyColor,
            (false, true) => LedMode::OnlyIntensity,
            (false, false) => bail!("You must choose a mode !"),
            _ => bail!("Only one mode can be active at a time !"),
        };

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
            mode,
            socket,
            audio_device,
            tui,
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
        socket.write_u8(self.mode.int_value())?;

        Ok(())
    }

    pub fn create_audio_stream(&self, size: usize) -> Result<(Stream, Consumer<i16>)> {
        let config = self.audio_device.default_input_config()?;

        let (mut prod, cons) = RingBuffer::new(size).split();

        let reader = self.audio_device.build_input_stream(
            &config.into(),
            move |data: &[i16], info| {
                prod.push_slice(data);
            },
            |e| eprintln!("CPAL Error: {:?}", e),
        )?;

        Ok((reader, cons))
    }

    pub fn draw(
        &mut self,
        raw_data: &[(f64, f64)],
        fft_data: &[(f64, f64)],
        intensity: f64,
        max_intensity: f64,
        classes: &[f64],
    ) {
        self.tui
            .draw(|frame| {
                let main_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Min(10),
                            Constraint::Length((3 * (classes.len() + 2) + classes.len()) as u16),
                        ]
                        .as_ref(),
                    )
                    .split(frame.size());

                let graph_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(main_layout[0]);

                let raw_graph = {
                    let raw_dataset = Dataset::default()
                        .name("Raw PCM")
                        .marker(Marker::Braille)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(Color::LightGreen))
                        .data(raw_data);

                    Chart::new(vec![raw_dataset])
                        .block(Block::default().title(" PCM Data ").borders(Borders::ALL))
                        .x_axis(Axis::default().bounds([0.0, raw_data.len() as f64]))
                        .y_axis(Axis::default().bounds([-2000.0, 2000.0]))
                };

                let fft_graph = {
                    let fft_dataset = Dataset::default()
                        .name("FFT")
                        .marker(Marker::Braille)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(Color::LightBlue))
                        .data(fft_data);

                    Chart::new(vec![fft_dataset])
                        .block(Block::default().title(" FFT Data ").borders(Borders::ALL))
                        .x_axis(Axis::default().bounds([0.0, fft_data.len() as f64]))
                        .y_axis(Axis::default().bounds([-2000.0, 2000.0]))
                };

                let mut bars_data = vec![(" I ", intensity as _)];
                let names = classes
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!(" {} ", i))
                    .collect::<Vec<_>>();
                for (i, &class) in classes.iter().enumerate() {
                    bars_data.push((names[i].as_str(), class as _));
                }

                let bars = {
                    BarChart::default()
                        .block(
                            Block::default()
                                .title(" Output Data ")
                                .borders(Borders::ALL),
                        )
                        .bar_width(3)
                        .max(max_intensity as _)
                        .data(&bars_data)
                };

                frame.render_widget(raw_graph, graph_layout[0]);
                frame.render_widget(fft_graph, graph_layout[1]);
                frame.render_widget(bars, main_layout[1]);
            })
            .unwrap();
    }
}
