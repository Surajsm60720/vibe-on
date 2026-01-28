use rodio::Source;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoeffs {
    fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
}

struct BiquadFilter {
    coeffs: BiquadCoeffs,
    filter_type: FilterType,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    fn new(filter_type: FilterType) -> Self {
        Self {
            coeffs: BiquadCoeffs::new(),
            filter_type,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn update_coeffs(&mut self, frequency: f32, sample_rate: u32, gain_db: f32, q: f32) {
        let w0 = 2.0 * PI * frequency / sample_rate as f32;
        let a = 10.0f32.powf(gain_db / 40.0);
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::LowShelf => {
                // Low Shelf: RBJ Cookbook
                // A = 10^(dB/40)  (Same A as peaking?) No.
                // RBJ: A  = 10^(dB/40) is consistent.
                // slope S = 1.
                // alpha = sin(w0)/2 * sqrt( (A + 1/A)*(1/S - 1) + 2 )
                // But simplified for Q? alpha = sin(w0)/(2*Q). Q usually 0.707 for shelf.

                // Let's use Q=0.707 for smooth shelf slope if using Q parameter.

                let sqrt_a = a.sqrt(); // Wait, usually formulas use A as 10^(dB/40).

                // RBJ formulas for Low Shelf
                let sa = 2.0 * sqrt_a * alpha;

                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + sa);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - sa);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + sa;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - sa;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                // High Shelf
                let sqrt_a = a.sqrt();
                let sa = 2.0 * sqrt_a * alpha;

                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + sa);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - sa);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + sa;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - sa;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;

                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Simplify by dividing by a0 pre-emptively
        self.coeffs.b0 = b0 / a0;
        self.coeffs.b1 = b1 / a0;
        self.coeffs.b2 = b2 / a0;
        self.coeffs.a1 = a1 / a0;
        self.coeffs.a2 = a2 / a0;
    }

    fn process(&mut self, sample: f32) -> f32 {
        let output = self.coeffs.b0 * sample + self.coeffs.b1 * self.x1 + self.coeffs.b2 * self.x2
            - self.coeffs.a1 * self.y1
            - self.coeffs.a2 * self.y2;

        // Shift delay lines
        self.x2 = self.x1;
        self.x1 = sample;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}

pub struct Equalizer<I>
where
    I: Source<Item = f32>,
{
    input: I,
    filters: Vec<Vec<BiquadFilter>>,
    gains: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    current_channel: usize,
    frequencies: [f32; 10],
    cached_gains: Vec<f32>,
    update_counter: usize,
}

impl<I> Equalizer<I>
where
    I: Source<Item = f32>,
{
    pub fn new(input: I, gains: Arc<Mutex<Vec<f32>>>) -> Self {
        let sample_rate = input.sample_rate();
        let channels = input.channels();

        // Standard 10 band ISO frequencies
        let frequencies = [
            31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ];

        let mut filters = Vec::with_capacity(channels as usize);
        for _ in 0..channels {
            let mut channel_filters = Vec::with_capacity(10);
            for (i, _) in frequencies.iter().enumerate() {
                let filter_type = if i == 0 {
                    FilterType::LowShelf
                } else if i == 9 {
                    FilterType::HighShelf
                } else {
                    FilterType::Peaking
                };
                channel_filters.push(BiquadFilter::new(filter_type));
            }
            filters.push(channel_filters);
        }

        let mut eq = Self {
            input,
            filters,
            gains,
            sample_rate,
            channels,
            current_channel: 0,
            frequencies,
            cached_gains: vec![0.0; 10],
            update_counter: 0,
        };

        eq.recalculate_coeffs();
        eq
    }

    fn recalculate_coeffs(&mut self) {
        if let Ok(gains) = self.gains.lock() {
            if gains.len() == 10 {
                // Check if actually changed to avoid spam?
                // Caller checks before calling, but we update cached_gains here.
                if self.cached_gains != *gains {
                    println!("[Equalizer] Updating gains: {:?}", gains);
                    self.cached_gains = gains.clone();
                }
            }
        }

        // Q Factor: Lower Q = Wider Bandwidth.
        // 1.0 is a reasonable balance for peaking filters.
        let q = 1.0;

        for channel_idx in 0..self.channels as usize {
            for (band_idx, filter) in self.filters[channel_idx].iter_mut().enumerate() {
                // For Shelving filters, Q is usually different (0.707 for Butterworth slope).
                // But reusing q=1.0 is acceptable or we can pass slope.
                // Our update_coeffs handles shelves using alpha derived from Q.
                filter.update_coeffs(
                    self.frequencies[band_idx],
                    self.sample_rate,
                    self.cached_gains[band_idx],
                    q,
                );
            }
        }
    }
}

impl<I> Iterator for Equalizer<I>
where
    I: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.update_counter += 1;
        if self.update_counter > 1000 {
            let mut changed = false;
            if let Ok(lock) = self.gains.try_lock() {
                if *lock != self.cached_gains {
                    changed = true;
                }
            }
            if changed {
                self.recalculate_coeffs();
            }
            self.update_counter = 0;
        }

        let sample = self.input.next()?;

        // No conversion needed, sample is f32
        let mut processed_sample = sample;

        if self.current_channel < self.filters.len() {
            let channel_filters = &mut self.filters[self.current_channel];
            for filter in channel_filters.iter_mut() {
                processed_sample = filter.process(processed_sample);
            }
        }

        self.current_channel = (self.current_channel + 1) % (self.channels as usize);

        Some(processed_sample)
    }
}

impl<I> Source for Equalizer<I>
where
    I: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }
}
