use anyhow::bail;
use rswave_remote::app::App;
use std::time::Duration;
use tokio::sync::oneshot::error::TryRecvError;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = App::new().await?;
    let mut app = app.lock();

    let (sender, mut ctrl_c_receiver) = tokio::sync::oneshot::channel();
    let ctrl_c_handle = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for Ctrl+C");
        sender.send(true).expect("Failed to send Ctrl+C ok");
    });

    app.start_recording()?;

    loop {
        match ctrl_c_receiver.try_recv() {
            Err(TryRecvError::Empty) => {
                // Ok, continue the loop
                if app.can_run() {
                    app.run_once().await?;
                    app.draw();
                } else {
                    tokio::time::delay_for(Duration::from_millis(10)).await;
                }
            }
            Ok(true) => {
                // We need to exit
                break;
            }
            _ => bail!("Something went wrong waiting for Ctrl+C !"),
        }
    }

    ctrl_c_handle.await?;

    app.cleanup()?;
    Ok(())
}
