use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use std::{cmp::Ordering, collections::VecDeque, f64::consts::PI, sync::Arc};

pub const DEFAULT_SAMPLE_SIZE: usize = 2048;
pub const DEFAULT_DELTA_HISTORY_SIZE: usize = 200;
pub const COMPRESSION_CONST: f64 = 1000.0;

// Use f64 because TUI graphs expect f64 anyway, and we can afford it.
pub struct AudioProcessor {
    sample_size: usize,
    delta_history_size: usize,

    fft_planner: RealFftPlanner<f64>,
    fft: Arc<dyn RealToComplex<f64>>,

    window: Vec<f64>,
    peak_input: f64,
    peak_output: f64,
    peak_delta: f64,
    peaks: Vec<f64>,

    input: Vec<f64>,
    raw_data_left: Vec<f64>,
    raw_data_right: Vec<f64>,
    fft_scratch: Vec<Complex<f64>>,
    fft_data_left: Vec<Complex<f64>>,
    fft_data_right: Vec<Complex<f64>>,

    output: Vec<f64>,
    prev_output: Vec<f64>,

    novelty_curve: VecDeque<f64>,
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_SIZE, DEFAULT_DELTA_HISTORY_SIZE)
    }
}

impl AudioProcessor {
    /// Create a new [AudioProcessor].
    /// It will automatically create and manage the buffers required for the analysis.
    pub fn new(sample_size: usize, delta_history_size: usize) -> Self {
        let mut fft_planner = RealFftPlanner::new();
        let fft = fft_planner.plan_fft_forward(sample_size);

        let mut processor = Self {
            sample_size,
            delta_history_size,

            fft_planner,
            fft,

            window: vec![],
            peak_input: 0.0,
            peak_output: 0.0,
            peak_delta: 0.0,
            peaks: vec![],

            input: vec![],
            raw_data_left: vec![],
            raw_data_right: vec![],
            fft_scratch: vec![],
            fft_data_left: vec![],
            fft_data_right: vec![],

            output: vec![],
            prev_output: vec![],

            novelty_curve: {
                let mut queue = VecDeque::with_capacity(delta_history_size);
                queue.resize(delta_history_size, 0.0);
                queue
            },
        };
        processor.recreate_fft();
        processor
    }

    pub fn sample_size(&self) -> usize {
        self.sample_size
    }

    pub fn set_sample_size(&mut self, sample_size: usize) {
        self.sample_size = sample_size;
        self.recreate_fft();
    }

    pub fn input(&mut self) -> &mut [f64] {
        &mut self.input
    }

    pub fn peak_input(&self) -> f64 {
        self.peak_input
    }

    pub fn peak_output(&self) -> f64 {
        self.peak_output
    }

    pub fn peak_delta(&self) -> f64 {
        self.peak_delta
    }

    pub fn peaks(&self) -> &[f64] {
        &self.peaks
    }

    pub fn output(&self) -> &[f64] {
        &self.output[1..]
    }

    pub fn novelty_curve(&self) -> impl Iterator<Item = f64> + '_ {
        self.novelty_curve.iter().copied()
    }

    pub fn novelty(&self) -> f64 {
        *self.novelty_curve.back().unwrap_or(&0.0)
    }

    pub fn novelty_peak(&self) -> f64 {
        self.novelty_curve
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0.0)
    }
}

impl AudioProcessor {
    /// Plan FFT and create buffers and window of the correct sizes.
    fn recreate_fft(&mut self) {
        self.fft = self.fft_planner.plan_fft_forward(self.sample_size);

        self.raw_data_left = self.fft.make_input_vec();
        self.raw_data_right = self.fft.make_input_vec();

        self.fft_scratch = self.fft.make_scratch_vec();

        self.fft_data_left = self.fft.make_output_vec();
        self.fft_data_right = self.fft.make_output_vec();

        self.input = vec![0.0; self.raw_data_left.len() + self.raw_data_right.len()];
        self.output = vec![0.0; self.fft_data_left.len()];
        self.prev_output = vec![0.0; self.output.len()];

        self.peaks = vec![0.0; self.output.len()];

        // Hann window
        self.window = (0..self.raw_data_left.len())
            .into_iter()
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f64 / (self.sample_size as f64 - 1.0)).cos()))
            .collect();
    }

    pub fn process(&mut self) {
        // Save output
        self.prev_output.copy_from_slice(&self.output);

        // Separate stereo channels and apply window
        for (i, samples) in self.input.chunks_exact_mut(2).enumerate() {
            // Also modify input so we can see the window being applied in the visualisation
            samples[0] *= self.window[i];
            samples[1] *= self.window[i];

            self.raw_data_left[i] = samples[0];
            self.raw_data_right[i] = samples[1];

            // Update peak
            self.peak_input = self.peak_input.max(samples[0]).max(samples[1]);
        }

        // Process
        // We unwrap because we now that the buffers are of the correct length
        self.fft
            .process_with_scratch(
                &mut self.raw_data_left,
                &mut self.fft_data_left,
                &mut self.fft_scratch,
            )
            .unwrap();
        self.fft
            .process_with_scratch(
                &mut self.raw_data_right,
                &mut self.fft_data_right,
                &mut self.fft_scratch,
            )
            .unwrap();

        // Build output
        let scale_coeff = 1.0 / (self.fft_data_left.len() as f64).sqrt();
        for (i, (left, right)) in self
            .fft_data_left
            .iter()
            .zip(self.fft_data_right.iter())
            .enumerate()
        {
            // Normalize and combine channels
            // Average L/R
            let mut val = (left.scale(scale_coeff).norm() + right.scale(scale_coeff).norm()) / 2.0;

            // Logarithmic compression
            val = (COMPRESSION_CONST * val).ln_1p();

            // Record peaks
            if val > self.peaks[i] {
                self.peaks[i] = val;

                if val > self.peak_output {
                    self.peak_output = val;
                }
            }

            self.output[i] = val;
        }

        // Novelty curve
        let mut novelty = 0.0;
        for (i, val) in self.output.iter().copied().enumerate() {
            let delta = (val - self.prev_output[i]).max(0.0);
            novelty += delta;
        }

        if self.novelty_curve.len() == self.delta_history_size {
            self.novelty_curve.pop_front();
        }
        self.novelty_curve.push_back(novelty);
    }
}
