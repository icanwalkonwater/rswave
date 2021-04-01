use log::{debug, info};
#[cfg(feature = "controller_gpio")]
use rswave_server::led_controllers::ControllerGpio;
#[cfg(feature = "controller_ws2811")]
use rswave_server::led_controllers::ControllerWs2811;
use rswave_server::{app::App, led_controllers::LedController, LedStripType, Opt};
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("Starting...");

    // Parse cmdline
    let opt: Opt = Opt::from_args();

    match opt.led_type {
        LedStripType::Ws2811 => {
            info!("Choosed led type WS2811");
            #[cfg(not(feature = "controller_ws2811"))]
            eprintln!("LED type WS2811 is not supported by this build !");
            #[cfg(feature = "controller_ws2811")]
            run_app(
                opt,
                ControllerWs2811::new(opt.led_count.unwrap(), opt.brightness)?,
            )?;
        }
        LedStripType::Gpio => {
            info!("Choosed led type GPIO");
            #[cfg(not(feature = "controller_gpio"))]
            eprintln!("LED type GPIO is not supported by this build !");
            #[cfg(feature = "controller_gpio")]
            run_app(
                opt,
                ControllerGpio::new(opt.pwm_freq, opt.pin_red, opt.pin_green, opt.pin_blue)?,
            )?;
        }
    }

    Ok(())
}

fn run_app<C: LedController + Send + 'static>(opt: Opt, mut controller: C) -> anyhow::Result<()> {
    if opt.reset {
        debug!("Reset and exit");
        controller.reset()?;
        return Ok(());
    }

    let mut app = App::new(opt, controller)?;
    loop {
        app.run()?;
        // TODO: listen for key inputs
    }
    app.stop()
}
