use crate::Opt;
use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use std::{collections::VecDeque, f64::consts::PI, sync::Arc};

pub struct AudioProcessor {
    sample_size: usize,
    novelty_size_st: usize,
    compression: f64,

    fft_planner: RealFftPlanner<f64>,
    fft: Arc<dyn RealToComplex<f64>>,

    window: Box<[f64]>,
    input: Box<[f64]>,
    // Stereo L/R
    raw_data: (Box<[f64]>, Box<[f64]>),
    fft_scratch: Box<[Complex<f64>]>,
    // Stereo L/R
    fft_data: (Box<[Complex<f64>]>, Box<[Complex<f64>]>),

    peak_input: f64,
    output: Box<[f64]>,
    prev_output: Box<[f64]>,

    novelty_curve: VecDeque<f64>,
}

impl AudioProcessor {
    pub fn new(opt: Opt) -> Self {
        assert!(
            opt.novelty_size >= opt.novelty_size_st,
            "Novelty size but be >= Short term novelty size !"
        );

        let mut fft_planner = RealFftPlanner::new();
        let fft = fft_planner.plan_fft_forward(opt.sample_size);

        let raw_data = (
            fft.make_input_vec().into_boxed_slice(),
            fft.make_input_vec().into_boxed_slice(),
        );
        let fft_scratch = fft.make_scratch_vec().into_boxed_slice();
        let fft_data = (
            fft.make_output_vec().into_boxed_slice(),
            fft.make_output_vec().into_boxed_slice(),
        );

        let input = vec![0.0; raw_data.0.len() + raw_data.1.len()].into_boxed_slice();
        let output = vec![0.0; fft_data.0.len()].into_boxed_slice();
        let prev_output = vec![0.0; output.len()].into_boxed_slice();

        // Hann window
        let window = (0..raw_data.0.len())
            .into_iter()
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f64 / (opt.sample_size as f64 - 1.0)).cos()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let mut novelty_curve = VecDeque::with_capacity(opt.novelty_size);
        novelty_curve.resize(opt.novelty_size, 0.0);

        Self {
            sample_size: opt.sample_size,
            novelty_size_st: opt.novelty_size_st,
            compression: opt.spectrum_compression,

            fft_planner,
            fft,

            window,
            input,
            raw_data,
            fft_scratch,
            fft_data,

            peak_input: 0.0,
            output,
            prev_output,

            novelty_curve,
        }
    }

    /// Probably the most important function of the program.
    pub fn process(&mut self) {
        // Save output
        self.prev_output.copy_from_slice(&self.output);

        // Separate channels and apply window
        for (i, samples) in self.input.chunks_exact_mut(2).enumerate() {
            // Apply window to the input so we can see it in the visualization
            samples[0] *= self.window[i];
            samples[1] *= self.window[i];

            self.raw_data.0[i] = samples[0];
            self.raw_data.1[i] = samples[0];

            // Update peak
            self.peak_input = self.peak_input.max(samples[0]).max(samples[1]);
        }

        // Process
        // We can unwrap because it errors only if the buffers aren't of the correct size
        self.fft
            .process_with_scratch(
                &mut self.raw_data.0,
                &mut self.fft_data.0,
                &mut self.fft_scratch,
            )
            .unwrap();
        self.fft
            .process_with_scratch(
                &mut self.raw_data.1,
                &mut self.fft_data.1,
                &mut self.fft_scratch,
            )
            .unwrap();

        // Post-process spectrum
        let scale_coeff = 1.0 / (self.fft_data.0.len() as f64).sqrt();
        for (i, (left, right)) in self
            .fft_data
            .0
            .iter()
            .zip(self.fft_data.1.iter())
            .enumerate()
        {
            // Normalize values them combine them
            // Average L/R
            let mut val = (left.scale(scale_coeff).norm() + right.scale(scale_coeff).norm()) / 2.0;

            // Logarithmic compression
            val = (self.compression * val).ln_1p();
        }

        // Novelty curve
        let novelty = self
            .output
            .iter()
            .copied()
            .enumerate()
            .map(|(i, val)| (val - self.prev_output[i]).max(0.0))
            .sum();

        self.novelty_curve.pop_front();
        self.novelty_curve.push_back(novelty);
    }
}
