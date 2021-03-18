use byteorder::{ReadBytesExt, WriteBytesExt};
use cichlid::{prelude::RainbowFillSingleCycle, ColorRGB};
use int_enum::IntEnum;
use rpi_led_common::{LedMode, MAGIC};
use rpi_led_local::{
    create_led_controller,
    runners::{ColorOnlyRunner, IntensityOnlyRampRunner, Runner},
    ControllerExt, LED_CHANNEL, LED_COUNT,
};
use rs_ws281x::Controller;
use std::{
    io::Write,
    net::{Ipv4Addr, TcpListener, TcpStream},
    thread::sleep,
    time::Duration,
};
use structopt::StructOpt;

#[derive(Copy, Clone, Debug, StructOpt)]
struct Opt {
    /// Port to use
    #[structopt(short, long, default_value = "20200")]
    port: u16,

    /// Brightness
    #[structopt(short, long, default_value = "100")]
    brightness: u8,

    /// Reset the led and exit
    #[structopt(short, long)]
    reset: bool,

    /// Whether or not to exit after the first connection has ended
    #[structopt(short, long)]
    multiple: bool,

    /// Just do something with the led for a while
    #[structopt(short, long)]
    demo: bool,
}

fn main() -> anyhow::Result<()> {
    // Parse cmdline
    let opt = Opt::from_args();

    let mut controller = create_led_controller()?;
    controller.set_brightness(LED_CHANNEL, opt.brightness);
    controller.set_all(ColorRGB::Black)?;

    if opt.reset {
        controller.set_all(ColorRGB::Black)?;
        return Ok(());
    } else if opt.demo {
        handle_demo(controller)?;
        return Ok(());
    }

    // Socket
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, opt.port))?;
    listener.set_nonblocking(false)?;

    if opt.multiple {
        loop {
            println!("Listening on {}...", listener.local_addr()?);
            let (socket, _) = listener.accept()?;
            println!("Connected to {}", socket.peer_addr()?);

            // Block until the connection is over
            // In other words: 1 connection at a time
            handle_connection(opt, socket, &mut controller)?;
        }
    } else {
        let (socket, _) = listener.accept()?;
        socket.set_nodelay(true)?;
        handle_connection(opt, socket, &mut controller)?;
    }

    Ok(())
}

fn handle_demo(mut controller: Controller) -> anyhow::Result<()> {
    let mut colors = [ColorRGB::Black; LED_COUNT as _];
    let mut hue = 0;
    loop {
        colors.rainbow_fill_single_cycle(hue);
        controller.set_all_individual(&colors)?;

        hue = hue.wrapping_add(5);
        sleep(Duration::from_millis(100));
    }
}

fn handle_connection(
    _opt: Opt,
    mut socket: TcpStream,
    controller: &mut Controller,
) -> anyhow::Result<()> {
    // Hello
    socket.write_u8(MAGIC)?;
    socket.flush()?;

    // Read mode
    let mode: LedMode = LedMode::from_int(socket.read_u8()?)?;

    match mode {
        LedMode::OnlyColor => {
            println!("Only color runner");
            let runner = ColorOnlyRunner::new(&mut socket)?;
            runner.run(socket, controller)?;
        }
        LedMode::OnlyIntensity => {
            println!("Only intensity runner");
            let runner = IntensityOnlyRampRunner::new(&mut socket)?;
            runner.run(socket, controller)?;
        }
        LedMode::ColorAndIntensity => {
            todo!()
        }
    };

    Ok(())
}
