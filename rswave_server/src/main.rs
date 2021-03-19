use log::{debug, info};
use rswave_server::{
    app::App,
    led_controllers::{ControllerWs2811, LedController},
    LedStripType, Opt,
};
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
            run_app(opt, ControllerWs2811::new(opt.led_count, opt.brightness)?)?;
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
