use rpi_led_remote::app::App;
use std::thread::sleep;
use std::time::{Duration, Instant};
use cpal::traits::StreamTrait;
use parking_lot::Mutex;
use cpal::StreamInstant;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

fn main() -> anyhow::Result<()> {
    let mut app = App::new()?;
    let mut app = app.lock();

    app.init_network()?;

    let (stream, mut cons) = app.create_audio_stream()?;

    stream.play()?;

    let mut planner = FftPlanner::new();

    let mut raw_data = Vec::new();
    let mut fft_data = Vec::new();
    let mut fft_data_display = Vec::new();
    loop {
        if !cons.is_empty() {
            raw_data.clear();
            raw_data.reserve_exact(cons.len());

            // Raw data
            cons.pop_each(|sample| {
                raw_data.push((raw_data.len() as f64, sample as f64));
                true
            }, Some(2000));

            // FFT
            fft_data.clear();
            fft_data.reserve_exact(raw_data.len());

            for (re, im) in raw_data.iter() {
                fft_data.push(Complex { re: *re, im: *im })
            }

            let fft = planner.plan_fft_forward(raw_data.len());
            fft.process(&mut fft_data);

            fft_data_display.clear();
            fft_data_display.reserve_exact(fft_data.len());

            for complex in fft_data.iter() {
                let val = complex.scale(1.0 / (fft_data.len() as f64).sqrt());
                fft_data_display.push((fft_data_display.len() as f64, val.re));
            }

            app.draw(&raw_data, &fft_data_display);
        }

        // sleep(Duration::from_millis(100));
    }

    Ok(())
}