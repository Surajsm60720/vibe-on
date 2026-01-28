use super::reverb::Freeverb;
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

// ... (previous imports)

// Add new fields to Equalizer struct
pub struct Equalizer<I>
where
    I: Source<Item = f32>,
{
    input: I,
    filters: Vec<Vec<BiquadFilter>>,
    gains: Arc<Mutex<Vec<f32>>>,
    // New DSP params (using Arc<Mutex> for thread safety if updated dynamically)
    // For simplicity, we can read them from the same gains Vector? No, stick to separate or extend vector.
    // Let's extend the shared vector? Or add a new struct.
    // The user wants "Preamp, Balance, etc."
    // Let's define specific indices in the gains vector?
    // Current gains vector is 10 bands.
    // We can add "extra" slots at the end (10 -> Preamp, 11 -> Balance, 12 -> Width).
    // This is the easiest way without changing the signature too much.
    // Indices:
    // 0-9: EQ Bands
    // 10: Preamp (dB)
    // 11: Balance (-1.0 Left to 1.0 Right)
    // 12: Stereo Width (0.0 Mono to 1.0 Normal to 2.0+ Wide)
    // 13+: Future?
    sample_rate: u32,
    channels: u16,
    current_channel: usize,
    frequencies: [f32; 10],
    cached_gains: Vec<f32>,
    update_counter: usize,

    // Buffering for Stereo Process
    pending_sample: Option<f32>,
    reverb: Freeverb,
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

        // Initialize cached gains with enough space for extended params
        // Default size is 10, but we want up to 15 to be safe.
        // We will resize cached_gains in recalculate if needed.
        // Or simply force it here.
        let mut eq = Self {
            input,
            filters,
            gains,
            sample_rate,
            channels,
            current_channel: 0,
            frequencies,
            cached_gains: vec![0.0; 15],
            update_counter: 0,
            pending_sample: None,
            reverb: Freeverb::new(sample_rate as u32),
        };

        eq.recalculate_coeffs();
        eq
    }

    fn recalculate_coeffs(&mut self) {
        if let Ok(gains) = self.gains.lock() {
            // Check if different. Gains might be 10 or more.
            // We only care about first 10 for EQ, others for DSP.
            // Just copy what we have.
            if gains.len() >= 10 {
                // Ensure cached_gains is large enough
                if self.cached_gains.len() < gains.len() {
                    self.cached_gains.resize(gains.len(), 0.0);
                }

                // Copy all
                for (i, &g) in gains.iter().enumerate() {
                    self.cached_gains[i] = g;
                }
            }
        }

        let q = 1.0;
        // Update Filters (Only first 10 params)
        for channel_idx in 0..self.channels as usize {
            for (band_idx, filter) in self.filters[channel_idx].iter_mut().enumerate() {
                let gain = if band_idx < self.cached_gains.len() {
                    self.cached_gains[band_idx]
                } else {
                    0.0
                };

                filter.update_coeffs(self.frequencies[band_idx], self.sample_rate, gain, q);
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
        // Handle pending sample from stereo pair
        if let Some(sample) = self.pending_sample.take() {
            return Some(sample);
        }

        self.update_counter += 1;
        if self.update_counter > 1000 {
            // Simplified check: Just recalculate every now and then or check lock
            // Optimization: Only lock if we suspect change? No, polling is fine for audio thread
            // if lock contention is low. 1000 samples is ~20ms at 48k. Good enough.
            self.recalculate_coeffs();
            self.update_counter = 0;
        }

        // --- Fetch Samples ---
        let mut left = self.input.next()?;

        // If Stereo, fetch right immediately for DSP
        let mut right = if self.channels == 2 {
            match self.input.next() {
                Some(s) => s,
                None => return Some(left), // Should not happen easily but safety
            }
        } else {
            0.0 // Dummy for mono logic if needed, but mono logic branches
        };

        // --- EQ Processing ---
        // Apply Filters to Left
        // Note: current_channel tracking logic in original code was:
        // 1 sample -> process channel 0 -> increment
        // 1 sample -> process channel 1 -> increment
        // But here we pulled TWO samples if stereo.
        // So we process channel 0 for left, channel 1 for right.

        if self.channels == 2 {
            // Process Left (Channel 0)
            if 0 < self.filters.len() {
                for filter in self.filters[0].iter_mut() {
                    left = filter.process(left);
                }
            }
            // Process Right (Channel 1)
            if 1 < self.filters.len() {
                for filter in self.filters[1].iter_mut() {
                    right = filter.process(right);
                }
            }
        } else {
            // Mono / Multi-channel generic fallback (old logic)
            // If mono, we just have 'left'.
            if self.current_channel < self.filters.len() {
                for filter in self.filters[self.current_channel].iter_mut() {
                    left = filter.process(left);
                }
            }
            // Update channel pointer for next call
            self.current_channel = (self.current_channel + 1) % (self.channels as usize);
            return Some(left);
        }

        // --- Extended DSP Effects ---
        // Indices: 10: Preamp, 11: Balance, 12: Stereo Width

        // 1. Preamp (Gain)
        let preamp_db = *self.cached_gains.get(10).unwrap_or(&0.0);
        if preamp_db != 0.0 {
            let factor = 10.0f32.powf(preamp_db / 20.0);
            left *= factor;
            right *= factor;
        }

        // 2. Stereo Width (Mid-Side)
        // Default 1.0 (Normal). 0.0 (Mono). >1.0 (Wide).
        // If param missing, default to 0.0? No, default should be 1.0?
        // We initialized vector to 0.0.
        // Wait, if default vector is 0s, Preamp=0dB, Balance=0 (Center), Width=0 (MONO?!).
        // Problem: Width 0.0 implies Mono. We want "Normal" to be 0 for UI slider?
        // Or we map UI 0..1..2 to 0..1..2?
        // Let's adopt UI standard:
        // Slider 0 (center) -> Effect 1.0 (Normal) ?
        // Or Slider 0% -> Mono, 50% -> Normal, 100% -> Wide?
        // Let's assume the parameter stored is the raw factor.
        // We need to ensure default in Store is 1.0 for Width.
        let width_factor = *self.cached_gains.get(12).unwrap_or(&1.0);

        if (width_factor - 1.0).abs() > 0.01 {
            let mid = (left + right) * 0.5;
            let side = (left - right) * 0.5;
            let new_side = side * width_factor;
            left = mid + new_side;
            right = mid - new_side;
        }

        // 3. Balance
        // -1.0 (Left) to 1.0 (Right). 0.0 Center.
        let balance = *self.cached_gains.get(11).unwrap_or(&0.0);
        if balance != 0.0 {
            // Pan law: Constant Power or Linear?
            // Simple linear for MVP:
            // If < 0 (Left biased): Left 1.0, Right 1.0+bal (decays to 0)
            if balance < 0.0 {
                right *= 1.0 + balance; // balance is negative
            } else {
                left *= 1.0 - balance;
            }
        }

        // 4. Reverb
        // Indices: 13: Mix (0.0 - 1.0), 14: Decay (0.0 - 1.0)
        let reverb_mix = *self.cached_gains.get(13).unwrap_or(&0.0);
        let reverb_decay = *self.cached_gains.get(14).unwrap_or(&0.5);

        if reverb_mix > 0.0 {
            // Update params only if changed (simple check usually ok, or update always as it's cheap setter)
            self.reverb.set_room_size(reverb_decay);
            self.reverb.set_wet(reverb_mix);
            self.reverb.set_dry(1.0 - (reverb_mix * 0.5)); // Slight dry dip when wet increases? Or keep dry 1.0?
                                                           // Freeverb usually mixes wet + dry. Let's keep dry constant or linear.
                                                           // set_dry(1.0) keeps original signal strong.
            self.reverb.set_dry(1.0);

            let (rev_l, rev_r) = self.reverb.process(left, right);
            left = rev_l;
            right = rev_r;
        }

        // --- Output ---
        self.pending_sample = Some(right);
        Some(left)
    }
}

// ... (Source implementation remains same)

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
