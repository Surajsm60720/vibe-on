/// DSP Feature Extraction Module
/// Computes low-level audio features for mood analysis

use rustfft::{num_complex::Complex, FftPlanner};

/// Compute RMS (Root Mean Square) energy of audio samples
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Compute Zero Crossing Rate (ratio of sign changes in audio)
/// Used to detect speech vs music
pub fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mut crossings = 0;
    for i in 1..samples.len() {
        if samples[i - 1] * samples[i] < 0.0 {
            crossings += 1;
        }
    }
    crossings as f32 / samples.len() as f32
}

/// Compute spectral centroid via FFT
/// Higher values indicate brighter, more high-frequency content
pub fn spectral_centroid(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    // Pad to next power of 2 for FFT efficiency
    let fft_size = samples.len().next_power_of_two();
    let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(fft_size);

    // Load samples with zero padding
    for &sample in samples.iter() {
        buffer.push(Complex { re: sample, im: 0.0 });
    }
    for _ in samples.len()..fft_size {
        buffer.push(Complex { re: 0.0, im: 0.0 });
    }

    // Compute FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    // Calculate spectral centroid: sum(bin_index * magnitude) / sum(magnitude)
    let mut numerator = 0.0;
    let mut denominator = 0.0;

    for (i, c) in buffer.iter().enumerate() {
        let mag = c.norm();
        numerator += i as f32 * mag;
        denominator += mag;
    }

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

/// Compute spectral rolloff (frequency below which 85% of energy is concentrated)
/// Helps distinguish bright vs dark sounds
pub fn spectral_rolloff(samples: &[f32], threshold: f32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let fft_size = samples.len().next_power_of_two();
    let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(fft_size);

    for &sample in samples.iter() {
        buffer.push(Complex { re: sample, im: 0.0 });
    }
    for _ in samples.len()..fft_size {
        buffer.push(Complex { re: 0.0, im: 0.0 });
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    let magnitudes: Vec<f32> = buffer.iter().map(|c| c.norm()).collect();
    let total_energy: f32 = magnitudes.iter().sum();

    if total_energy == 0.0 {
        return 0.0;
    }

    let mut cumulative = 0.0;
    let threshold_energy = total_energy * threshold;

    for (i, &mag) in magnitudes.iter().enumerate() {
        cumulative += mag;
        if cumulative >= threshold_energy {
            return i as f32;
        }
    }

    magnitudes.len() as f32
}

/// Compute chroma energy (12 pitch classes: C, C#, D, etc.)
/// Returns array of 12 values, one per semitone
pub fn chroma_features(samples: &[f32], sample_rate: f32) -> [f32; 12] {
    let mut chroma = [0.0; 12];

    if samples.is_empty() || sample_rate <= 0.0 {
        return chroma;
    }

    let fft_size = samples.len().next_power_of_two();
    let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(fft_size);

    for &sample in samples.iter() {
        buffer.push(Complex { re: sample, im: 0.0 });
    }
    for _ in samples.len()..fft_size {
        buffer.push(Complex { re: 0.0, im: 0.0 });
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    // Map FFT bins to chromatic bins
    let freq_resolution = sample_rate / fft_size as f32;
    let a4_freq = 440.0;

    for (bin, c) in buffer.iter().enumerate() {
        let freq = bin as f32 * freq_resolution;
        if freq == 0.0 || freq > sample_rate / 2.0 {
            continue;
        }

        // Convert frequency to cents from A0
        let cents = 1200.0 * (freq / a4_freq).log2();
        let cent_class = ((cents / 100.0).floor() as i32 % 12 + 12) % 12;

        if cent_class >= 0 && (cent_class as usize) < 12 {
            chroma[cent_class as usize] += c.norm();
        }
    }

    // Normalize
    let sum: f32 = chroma.iter().sum();
    if sum > 0.0 {
        for val in &mut chroma {
            *val /= sum;
        }
    }

    chroma
}

/// Detect dominant frequency using FFT
pub fn dominant_frequency(samples: &[f32], sample_rate: f32) -> f32 {
    if samples.is_empty() || sample_rate <= 0.0 {
        return 0.0;
    }

    let fft_size = samples.len().next_power_of_two();
    let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(fft_size);

    for &sample in samples.iter() {
        buffer.push(Complex { re: sample, im: 0.0 });
    }
    for _ in samples.len()..fft_size {
        buffer.push(Complex { re: 0.0, im: 0.0 });
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    let mut max_mag = 0.0;
    let mut max_bin = 0;

    for (bin, c) in buffer.iter().enumerate() {
        let mag = c.norm();
        if mag > max_mag {
            max_mag = mag;
            max_bin = bin;
        }
    }

    let freq_resolution = sample_rate / fft_size as f32;
    (max_bin as f32) * freq_resolution
}

/// Compute signal-to-noise ratio (simplified)
/// Uses relationship between RMS and noise floor
pub fn signal_to_noise_ratio(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let rms_val = rms(samples);
    let mut abs_samples: Vec<f32> = samples.iter().map(|x| x.abs()).collect();
    abs_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let noise_floor = if abs_samples.len() > 10 {
        abs_samples[abs_samples.len() / 10]
    } else {
        0.0
    };

    if noise_floor == 0.0 {
        1.0
    } else {
        20.0 * (rms_val / noise_floor).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms() {
        let samples = vec![0.5, 0.5, 0.5, 0.5];
        let result = rms(&samples);
        assert!((result - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_zero_crossing_rate() {
        let samples = vec![0.1, -0.1, 0.1, -0.1];
        let zcr = zero_crossing_rate(&samples);
        assert!(zcr > 0.0 && zcr <= 1.0);
    }

    #[test]
    fn test_chroma_normalization() {
        let samples: Vec<f32> = (0..1000).map(|i| ((i as f32 * 0.1).sin())).collect();
        let chroma = chroma_features(&samples, 44100.0);
        let sum: f32 = chroma.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }
}
