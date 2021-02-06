use std::net::TcpStream;
use crate::Opt;
use anyhow::Result;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use rpi_led_common::{MAGIC, LedMode, IntEnum};
use std::io::{Write, Stdout, stdout};
use cpal::traits::{DeviceTrait, HostTrait};
use anyhow::{anyhow, ensure, bail};
use std::fs::read;
use structopt::StructOpt;
use cpal::{InputCallbackInfo, Stream, StreamInstant};
use std::sync::{Arc, Barrier};
use tui::Terminal;
use tui::backend::CrosstermBackend;
use parking_lot::{Mutex, Condvar};
use ringbuf::{RingBuffer, Consumer};
use tui::widgets::{Block, Borders, Dataset, GraphType, Chart, Axis};
use tui::symbols::Marker;
use tui::style::{Style, Color};
use std::cmp::Ordering;

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

    pub fn create_audio_stream(&self) -> Result<(Stream, Consumer<i16>)> {
        let config = self.audio_device.default_input_config()?;

        let (mut prod, cons) = RingBuffer::new(40_000).split();

        let reader = self.audio_device.build_input_stream(
            &config.into(),
            move |data: &[i16], info| {
                prod.push_slice(data);
            },
            |e| eprintln!("CPAL Error: {:?}", e)
        )?;

        Ok((reader, cons))
    }

    pub fn draw(&mut self, raw_data: &[(f64, f64)], fft_data: &[(f64, f64)]) {
        self.tui.draw(|frame| {

            let raw_dataset = Dataset::default()
                .name("Raw PCM")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::LightGreen))
                .data(raw_data);

            let fft_dataset = Dataset::default()
                .name("FFT")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::LightBlue))
                .data(fft_data);

            let chart = Chart::new(vec![raw_dataset, fft_dataset])
                .block(Block::default().title("My chart").borders(Borders::ALL))
                .x_axis(Axis::default()
                    .title("i")
                    .bounds([0.0, raw_data.len() as f64]))
                .y_axis(Axis::default()
                    .title("PCM")
                    .bounds([-2000.0, 2000.0]));

            frame.render_widget(chart, frame.size());
        });
    }
}