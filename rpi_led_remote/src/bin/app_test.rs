use cpal::traits::StreamTrait;
use realfft::RealFftPlanner;
use rpi_led_remote::app::App;
use std::{
    cmp::Ordering,
    thread::sleep,
    time::{Duration, Instant},
};

fn main() -> anyhow::Result<()> {
    let app = App::new()?;
    let mut app = app.lock();

    app.init_network()?;

    const SAMPLE_SIZE: usize = 2048;

    let (stream, mut cons) = app.create_audio_stream(SAMPLE_SIZE * 2)?;

    stream.play()?;

    let mut planner = RealFftPlanner::new();
    let fft = planner.plan_fft_forward(SAMPLE_SIZE);

    let mut raw_data = fft.make_input_vec();
    let mut raw_data_display = Vec::new();
    let mut fft_data = fft.make_output_vec();
    let mut fft_data_display = Vec::new();
    let mut max_intensity = 0.0;

    const AMOUNT_CLASSES: usize = 16;
    const DISCARDED_FREQ: usize = 0;

    loop {
        let start = Instant::now();

        if cons.len() >= SAMPLE_SIZE {
            raw_data.clear();
            raw_data.reserve_exact(cons.len());

            // Raw data
            cons.pop_each(
                |sample| {
                    raw_data.push(sample as f64);
                    true
                },
                Some(SAMPLE_SIZE),
            );

            // Display raw data
            raw_data_display.clear();
            raw_data_display.reserve_exact(raw_data.len());

            for sample in raw_data.iter() {
                raw_data_display.push((raw_data_display.len() as _, *sample));
            }

            // FFT
            fft.process(&mut raw_data, &mut fft_data).unwrap();

            // Display FFT
            fft_data_display.clear();
            fft_data_display.reserve_exact(fft_data.len());

            for complex in fft_data.iter() {
                let val = complex.scale(1.0 / (fft_data.len() as f64).sqrt());
                fft_data_display.push((fft_data_display.len() as f64, val.re));
            }

            // Intensity
            let intensity =
                fft_data.iter().map(|c| c.re.abs()).sum::<f64>() / fft_data.len() as f64;
            let intensity = 10.0 * intensity.log10() - 10.0;

            // Separate classes

            // Combine stereo data
            let mut fft_combined = Vec::with_capacity(fft_data.len() / 2 - DISCARDED_FREQ);
            for (i, freq_left) in fft_data
                .iter()
                .copied()
                .enumerate()
                .take(fft_data.len() / 2 - DISCARDED_FREQ)
            {
                let freq_right = fft_data[fft_data.len() - i - 1];
                let freq_combined = (freq_left.re.abs() + freq_right.re.abs()) / 2.0;
                fft_combined.push(freq_combined);
            }

            // Compute classes
            let class_effective = fft_combined.len() / AMOUNT_CLASSES;
            let classes = fft_combined
                .chunks_exact(class_effective)
                .map(|chunk| chunk.iter().copied().sum::<f64>() / chunk.len() as f64)
                .map(|class| 10.0 * class.log10() - 10.0)
                .collect::<Vec<_>>();

            // Compute max for bars
            let max_class = classes
                .iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(max_intensity);
            if max_class > max_intensity {
                max_intensity = max_class;
            }

            app.draw(
                &raw_data_display,
                &fft_data_display,
                intensity,
                max_intensity,
                &classes,
            );
        }

        let time = Instant::now().duration_since(start).as_micros();

        sleep(Duration::from_millis(33));
    }
}
