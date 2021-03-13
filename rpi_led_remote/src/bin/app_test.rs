use rpi_led_remote::app::App;
use std::{
    thread::sleep,
    time::{Duration},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = App::new().await?;
    let mut app = app.lock();

    app.init_network()?;
    app.recreate_audio_stream()?;
    app.start_recording()?;

    loop {
        if app.can_run() {
            app.run_once().await?;
            app.draw();
        } else {
            sleep(Duration::from_millis(10));
        }
    }
}
