//! FFT-based audio analysis for real-time visualization.
//!
//! This module provides audio spectrum analysis using the `rustfft` crate,
//! which is a pure Rust implementation with no platform-specific dependencies.
//! Cross-platform compatibility: Windows, macOS, Linux.

use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, RwLock};

/// FFT size for frequency analysis (power of 2)
pub const FFT_SIZE: usize = 512;

/// Number of frequency bins exposed to frontend (lower = more performant)
pub const NUM_FREQUENCY_BINS: usize = 64;

/// Number of waveform samples exposed to frontend
pub const WAVEFORM_SAMPLES: usize = 128;

/// Shared visualizer data that can be read by the Tauri command
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct VisualizerData {
    /// Normalized frequency bins (0.0 to 1.0), bass to treble
    pub frequency_bins: Vec<f32>,
    /// Waveform samples for oscilloscope display (-1.0 to 1.0)
    pub waveform: Vec<f32>,
}

/// Thread-safe FFT processor for audio visualization.
///
/// This processor maintains a ring buffer of audio samples and computes
/// FFT on-demand when `get_visualizer_data()` is called.
pub struct FftProcessor {
    /// Ring buffer for incoming samples (interleaved stereo)
    sample_buffer: Arc<RwLock<RingBuffer>>,
    /// Most recent computed visualizer data
    last_data: Arc<RwLock<VisualizerData>>,
    /// Sample rate for frequency calculations
    sample_rate: u32,
}

impl FftProcessor {
    /// Create a new FFT processor.
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate (e.g., 44100, 48000)
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_buffer: Arc::new(RwLock::new(RingBuffer::new(FFT_SIZE * 2))),
            last_data: Arc::new(RwLock::new(VisualizerData::default())),
            sample_rate,
        }
    }

    /// Get a clone of the sample buffer Arc for use by the audio wrapper.
    pub fn get_buffer_handle(&self) -> Arc<RwLock<RingBuffer>> {
        Arc::clone(&self.sample_buffer)
    }

    /// Compute FFT and return visualizer data.
    ///
    /// This is called by the Tauri command at ~60fps.
    pub fn get_visualizer_data(&self) -> VisualizerData {
        // Get samples from ring buffer
        let samples = {
            let buffer = self.sample_buffer.read().unwrap();
            let count = buffer.len();
            let s = buffer.get_samples(FFT_SIZE);
            // Debug: log sample count periodically
            static DEBUG_COUNTER: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);
            let counter = DEBUG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if counter % 120 == 0 {
                // Log every ~2 seconds at 60fps
                println!(
                    "[FFT] Buffer has {} samples, returning {} for FFT",
                    count,
                    s.len()
                );
            }
            s
        };

        if samples.is_empty() {
            return VisualizerData::default();
        }

        // Prepare FFT input with Hann window for smoother spectral analysis
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        let mut complex_input: Vec<Complex<f32>> = samples
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                // Apply Hann window to reduce spectral leakage
                let window = 0.5
                    * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
                Complex::new(sample * window, 0.0)
            })
            .collect();

        // Pad with zeros if not enough samples
        while complex_input.len() < FFT_SIZE {
            complex_input.push(Complex::new(0.0, 0.0));
        }

        // Perform FFT
        fft.process(&mut complex_input);

        // Convert to magnitude spectrum (only first half is useful - Nyquist)
        let half_size = FFT_SIZE / 2;
        let magnitudes: Vec<f32> = complex_input[..half_size]
            .iter()
            .map(|c| c.norm() / (FFT_SIZE as f32).sqrt())
            .collect();

        // Bin the frequencies into NUM_FREQUENCY_BINS using logarithmic scale
        // This gives more resolution to bass frequencies (more perceptually accurate)
        let frequency_bins = self.bin_frequencies(&magnitudes, half_size);

        // Get waveform samples (last N samples for oscilloscope)
        let waveform = {
            let buffer = self.sample_buffer.read().unwrap();
            let raw = buffer.get_samples(WAVEFORM_SAMPLES);
            raw.iter().map(|&s| s.clamp(-1.0, 1.0)).collect()
        };

        let data = VisualizerData {
            frequency_bins,
            waveform,
        };

        // Cache the data
        if let Ok(mut last) = self.last_data.write() {
            *last = data.clone();
        }

        data
    }

    /// Bin raw FFT magnitudes into display bins using logarithmic frequency scaling.
    fn bin_frequencies(&self, magnitudes: &[f32], half_size: usize) -> Vec<f32> {
        let mut bins = vec![0.0f32; NUM_FREQUENCY_BINS];

        if magnitudes.is_empty() || half_size == 0 {
            return bins;
        }

        // Logarithmic frequency binning for perceptual accuracy
        // Low frequencies get more bins, high frequencies get compressed
        let nyquist = self.sample_rate as f32 / 2.0;
        let freq_per_bin = nyquist / half_size as f32;

        // Frequency range we care about: 20Hz to 20kHz
        let min_freq = 20.0f32;
        let max_freq = (nyquist).min(20000.0);
        let log_min = min_freq.ln();
        let log_max = max_freq.ln();
        let log_range = log_max - log_min;

        for (bin_idx, bin) in bins.iter_mut().enumerate() {
            // Calculate frequency range for this display bin (logarithmic)
            let bin_start_log = log_min + (bin_idx as f32 / NUM_FREQUENCY_BINS as f32) * log_range;
            let bin_end_log =
                log_min + ((bin_idx + 1) as f32 / NUM_FREQUENCY_BINS as f32) * log_range;

            let freq_start = bin_start_log.exp();
            let freq_end = bin_end_log.exp();

            // Find FFT bins that fall in this frequency range
            let fft_bin_start = ((freq_start / freq_per_bin) as usize).min(half_size - 1);
            let fft_bin_end = ((freq_end / freq_per_bin) as usize).min(half_size);

            if fft_bin_end > fft_bin_start {
                // Average (or max) of FFT bins in this range
                let sum: f32 = magnitudes[fft_bin_start..fft_bin_end].iter().sum();
                let avg = sum / (fft_bin_end - fft_bin_start) as f32;

                // Normalize to 0-1 range (with some headroom for loud audio)
                // Apply slight compression for visual appeal
                *bin = (avg * 3.0).min(1.0);
            }
        }

        bins
    }

    /// Get the last computed visualizer data (for quick access without recomputing).
    pub fn get_last_data(&self) -> VisualizerData {
        self.last_data.read().unwrap().clone()
    }
}

impl Default for FftProcessor {
    fn default() -> Self {
        Self::new(44100)
    }
}

/// Simple ring buffer for storing audio samples.
pub struct RingBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    capacity: usize,
    filled: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write_pos: 0,
            capacity,
            filled: 0,
        }
    }

    /// Push a single sample to the buffer.
    pub fn push(&mut self, sample: f32) {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.filled = self.filled.saturating_add(1).min(self.capacity);
    }

    /// Push multiple samples to the buffer.
    pub fn push_samples(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.push(sample);
        }
    }

    /// Get the most recent N samples (for FFT/waveform).
    pub fn get_samples(&self, count: usize) -> Vec<f32> {
        let count = count.min(self.filled).min(self.capacity);
        if count == 0 {
            return vec![];
        }

        let mut result = Vec::with_capacity(count);

        // Calculate start position (going backwards from write position)
        let start = if self.write_pos >= count {
            self.write_pos - count
        } else {
            self.capacity - (count - self.write_pos)
        };

        for i in 0..count {
            let idx = (start + i) % self.capacity;
            result.push(self.buffer[idx]);
        }

        result
    }

    /// Get the number of samples currently in the buffer.
    pub fn len(&self) -> usize {
        self.filled
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }
}

/// A Source wrapper that taps audio samples for FFT analysis.
///
/// This wraps any rodio Source and copies samples to the FFT ring buffer
/// without modifying the audio output.
pub struct VisualizerTap<S>
where
    S: rodio::Source<Item = f32>,
{
    inner: S,
    buffer: Arc<RwLock<RingBuffer>>,
    channels: u16,
    sample_rate: u32,
}

impl<S> VisualizerTap<S>
where
    S: rodio::Source<Item = f32>,
{
    /// Create a new visualizer tap wrapping the given source.
    pub fn new(source: S, buffer: Arc<RwLock<RingBuffer>>) -> Self {
        let channels = source.channels();
        let sample_rate = source.sample_rate();

        Self {
            inner: source,
            buffer,
            channels,
            sample_rate,
        }
    }
}

impl<S> Iterator for VisualizerTap<S>
where
    S: rodio::Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;

        // Copy sample to FFT buffer (every sample for mono, or mix for stereo)
        if let Ok(mut buffer) = self.buffer.write() {
            buffer.push(sample);
        }

        Some(sample)
    }
}

impl<S> rodio::Source for VisualizerTap<S>
where
    S: rodio::Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer = RingBuffer::new(4);
        buffer.push(1.0);
        buffer.push(2.0);
        buffer.push(3.0);

        let samples = buffer.get_samples(3);
        assert_eq!(samples, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = RingBuffer::new(4);
        buffer.push(1.0);
        buffer.push(2.0);
        buffer.push(3.0);
        buffer.push(4.0);
        buffer.push(5.0); // Overwrites 1.0

        let samples = buffer.get_samples(4);
        assert_eq!(samples, vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_fft_processor_empty() {
        let processor = FftProcessor::new(44100);
        let data = processor.get_visualizer_data();

        // Should return empty/default data when no samples
        assert!(data.waveform.is_empty() || data.waveform.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_frequency_binning() {
        let processor = FftProcessor::new(44100);
        let magnitudes = vec![0.5; 256]; // Flat spectrum

        let bins = processor.bin_frequencies(&magnitudes, 256);

        // All bins should have some value
        assert_eq!(bins.len(), NUM_FREQUENCY_BINS);
        assert!(bins.iter().all(|&x| x > 0.0));
    }
}
