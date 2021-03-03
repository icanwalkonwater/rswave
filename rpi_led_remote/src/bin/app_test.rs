use rpi_led_remote::app::App;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

fn main() -> anyhow::Result<()> {
    let app = App::new()?;
    let mut app = app.lock();

    app.init_network()?;
    app.recreate_audio_stream()?;
    app.start_recording()?;

    loop {
        if app.can_run() {
            app.run_once()?;
            app.draw();
        }

        sleep(Duration::from_millis(50));
    }
}
