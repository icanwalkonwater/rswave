use cpal::traits::StreamTrait;
use realfft::RealFftPlanner;
use rpi_led_remote::app::App;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

fn main() -> anyhow::Result<()> {
    let app = App::new()?;
    let mut app = app.lock();

    app.init_network()?;

    const SAMPLE_SIZE: usize = 4096;

    let (stream, mut cons) = app.create_audio_stream(SAMPLE_SIZE * 2)?;

    stream.play()?;

    let mut planner = RealFftPlanner::new();
    let fft = planner.plan_fft_forward(SAMPLE_SIZE);

    let mut raw_data = fft.make_input_vec();
    let mut raw_data_display = vec![(0.0, 0.0); raw_data.len()];
    let mut fft_data = fft.make_output_vec();
    let mut fft_data_display = vec![(0.0, 0.0); fft_data.len() / 2];

    let mut max_data = 0.0;
    let mut max_intensity = 0.0;

    let amount_classes: usize = (SAMPLE_SIZE as f32 / 2.0 + 1.0).log2() as usize;

    loop {
        let start = Instant::now();

        if cons.len() >= SAMPLE_SIZE {
            raw_data.clear();

            // Raw data
            cons.pop_each(
                |sample| {
                    let sample = sample as f64;
                    if sample.abs() > max_data {
                        max_data = sample.abs() + 1000.0;
                    }
                    raw_data.push(sample);
                    true
                },
                Some(SAMPLE_SIZE),
            );

            // Display raw data
            for (i, sample) in raw_data.iter().enumerate() {
                raw_data_display[i] = (i as _, *sample);
            }

            // FFT
            fft.process(&mut raw_data, &mut fft_data).unwrap();

            // Display FFT
            for (i, complex) in fft_data.iter().enumerate().take(fft_data_display.len()) {
                let val = complex.scale(1.0 / (fft_data.len() as f64).sqrt());
                fft_data_display[i] = (i as _, val.norm().log10())
            }

            // Intensity
            let intensity =
                fft_data.iter().map(|c| c.re.abs()).sum::<f64>() / fft_data.len() as f64;
            let intensity = 10.0 * intensity.log10() - 10.0;

            // Compute buckets
            /*let mut buckets = vec![0.0; amount_classes];
            for (i, sample) in fft_data[1..].iter().enumerate() {
                let bucket_index = (i as f32).log2() as usize;
                let val = 1000.0 * (sample.norm().log10() - 2.0);
                if val > buckets[bucket_index] {
                    buckets[bucket_index] = val;
                }
            }*/
            let buckets_ranges = [19.0, 100.0, 400.0, 2600.0, 5200.0, f32::MAX];
            let mut buckets = vec![0.0; buckets_ranges.len()];

            let mut active_bucket = 0;
            for (i, sample) in fft_data.iter().take(fft_data_display.len()).enumerate() {
                let freq = i as f32 * 44100.0 / SAMPLE_SIZE as f32;
                if freq > buckets_ranges[active_bucket] {
                    active_bucket += 1;
                }

                let val = 1000.0 * (sample.norm().log10() - 2.0);
                if val > buckets[active_bucket] {
                    buckets[active_bucket] = val;
                }
            }

            let max_bucket = buckets.iter().copied().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
            if max_bucket > max_intensity {
                max_intensity = max_bucket;
            }

            // Compute classes
            /*let class_effective = fft_data.len() / amount_classes;
            let classes = fft_data
                .chunks_exact(class_effective)
                // .map(|chunk| chunk.iter().copied().map(|c| c.norm()).sum::<f64>() / chunk.len() as f64)
                .map(|chunk| chunk.iter().copied().max_by(|a, b| a.norm().partial_cmp(&b.norm()).unwrap()).unwrap().norm())
                .map(|class| 1000.0 * (class.log10() - 2.0))
                .collect::<Vec<_>>();*/

            // Compute max for bars
            /*let max_class = classes
                .iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(max_intensity);
            if max_class > max_intensity {
                max_intensity = max_class;
            }*/

            app.draw(
                &raw_data_display,
                &fft_data_display,
                intensity,
                max_data,
                max_intensity,
                &buckets,
            );
        }

        let time = Instant::now().duration_since(start).as_micros();

        sleep(Duration::from_millis(50));
    }
}
