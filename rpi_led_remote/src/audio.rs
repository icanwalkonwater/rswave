use std::mem::swap;

pub struct AudioProcessor {
    // Ring buffer
    history: Vec<f32>,
    current: usize,
}

impl AudioProcessor {
    pub fn new(mut history_size: usize) -> Self {
        if history_size % 2 != 0 {
            history_size += 1;
        }

        Self {
            history: vec![0.0; history_size],
            current: 0,
        }
    }

    fn compute_rms<D: Into<f32> + Copy>(data: &[D]) -> f32 {
        let mut temp: f32 = data
            .iter()
            .copied()
            .map(|sample| sample.into())
            .map(|sample| sample * sample)
            .sum();
        temp /= data.len() as f32;
        temp.sqrt()
    }

    fn compute_min_max(&self) -> (f32, f32) {
        let mut min = self.history[0];
        let mut max = min;

        for chunk in self.history.chunks_exact(2) {
            let mut first = chunk[0];
            let mut second = chunk[1];

            if first > second {
                swap(&mut first, &mut second);
            }

            if first < min {
                min = first;
            }
            if second > max {
                max = second;
            }
        }

        (min, max)
    }

    #[inline]
    fn interpolate(min: f32, max: f32, value: f32) -> f32 {
        (value - min) / (max - min)
    }

    pub fn update<D: Into<f32> + Copy>(&mut self, data: &[D]) -> f32 {
        let rms = Self::compute_rms(data);

        // Update buffer
        self.current += 1;
        self.current %= self.history.len();
        self.history[self.current] = rms;

        let (min, max) = self.compute_min_max();
        let relative_volume = Self::interpolate(min, max, rms);

        relative_volume
    }
}
