/// Rust-based Audio Feature Analyzer
/// Performs DSP analysis entirely in Rust without external dependencies

use super::audio_loader::load_audio_truncated;
use super::dsp::*;
use super::types::AudioFeatures;
use chrono::Local;

/// Analyze audio file using Rust DSP
/// Returns AudioFeatures matching the Essentia output format
pub fn analyze_audio_file_rust(path: &str) -> Result<AudioFeatures, String> {
    println!("[Rust Analyzer] Starting analysis of: {}", path);

    // Load audio (limit to first 3 minutes for performance)
    let (samples, metadata) = load_audio_truncated(path, Some(180.0))?;

    if samples.is_empty() {
        return Err("No audio samples loaded".to_string());
    }

    println!(
        "[Rust Analyzer] Processing {} samples at {} Hz",
        samples.len(),
        metadata.sample_rate
    );

    // Compute low-level features
    let rms_val = rms(&samples);
    let zcr_val = zero_crossing_rate(&samples);
    let sc = spectral_centroid(&samples);
    let _rolloff = spectral_rolloff(&samples, 0.85);
    let chroma = chroma_features(&samples, metadata.sample_rate as f32);
    let _dominant_freq = dominant_frequency(&samples, metadata.sample_rate as f32);

    // Estimate tempo using frame-based energy peaks (simplified BPM detection)
    let tempo = estimate_tempo(&samples, metadata.sample_rate);

    // Detect key from chroma vector (strongest bin = key)
    let (key, _key_strength) = detect_key(&chroma);

    // Estimate scale (major vs minor) - simplified heuristic
    let is_major_scale = estimate_major_scale(&chroma);

    // Compute mood features using heuristics (matching Python logic)
    let energy = compute_energy(rms_val, sc);
    let valence = compute_valence(tempo, is_major_scale, sc, energy);
    let danceability = compute_danceability(tempo, zcr_val);
    let acousticness = compute_acousticness(sc);
    let speechiness = compute_speechiness(zcr_val);
    let instrumentalness = compute_instrumentalness(speechiness);
    let liveness = compute_liveness(&samples);

    // Compute loudness in dB
    let loudness = if rms_val > 0.0 {
        20.0f64 * (rms_val as f64).log10()
    } else {
        -60.0
    };

    let features = AudioFeatures {
        valence: (valence * 10000.0).round() / 10000.0,
        energy: (energy * 10000.0).round() / 10000.0,
        danceability: (danceability * 10000.0).round() / 10000.0,
        tempo: (tempo * 100.0).round() / 100.0,
        key,
        loudness: (loudness * 100.0).round() / 100.0,
        instrumentalness: (instrumentalness * 10000.0).round() / 10000.0,
        acousticness: (acousticness * 10000.0).round() / 10000.0,
        speechiness: (speechiness * 10000.0).round() / 10000.0,
        liveness: (liveness * 10000.0).round() / 10000.0,
        analysis_version: 2, // Version 2 = Rust analyzer
        analyzed_at: Some(Local::now().to_rfc3339()),
        analysis_error: None,
        analysis_backend: Some("rust".to_string()),
    };

    println!("[Rust Analyzer] Analysis complete: {:?}", features);

    Ok(features)
}

/// Estimate tempo using frame-based energy detection
/// Simplified BPM detection - detects peaks in energy frames
fn estimate_tempo(samples: &[f32], sample_rate: u32) -> f64 {
    if samples.is_empty() {
        return 120.0; // Default fallback
    }

    let frame_size = sample_rate as usize / 10; // 100ms frames
    if frame_size < 2 {
        return 120.0;
    }

    let mut frame_energies = Vec::new();

    for chunk in samples.chunks(frame_size) {
        let energy = rms(chunk);
        frame_energies.push(energy);
    }

    if frame_energies.len() < 3 {
        return 120.0;
    }

    // Detect peaks in energy contour
    let mut peak_distances = Vec::new();
    let mut last_peak = None;

    for i in 1..frame_energies.len() - 1 {
        if frame_energies[i] > frame_energies[i - 1] && frame_energies[i] > frame_energies[i + 1] {
            if let Some(prev) = last_peak {
                peak_distances.push(i - prev);
            }
            last_peak = Some(i);
        }
    }

    if peak_distances.is_empty() {
        return 120.0;
    }

    // Average peak distance in frames, convert to BPM
    let avg_peak_distance = peak_distances.iter().sum::<usize>() as f64 / peak_distances.len() as f64;
    let beat_duration_seconds = avg_peak_distance * frame_size as f64 / sample_rate as f64;
    let bpm = (60.0 / beat_duration_seconds).clamp(60.0, 200.0);

    bpm
}

/// Detect musical key from chroma features
/// Returns (key: 0-11, strength: 0.0-1.0) where 0=C, 1=C#, etc.
fn detect_key(chroma: &[f32; 12]) -> (i32, f32) {
    let mut max_idx = 0;
    let mut max_energy = 0.0f32;

    for (idx, &energy) in chroma.iter().enumerate() {
        if energy > max_energy {
            max_energy = energy;
            max_idx = idx;
        }
    }

    (max_idx as i32, max_energy)
}

/// Estimate if scale is major or minor using chroma vector
/// Simplified heuristic based on chroma distribution
fn estimate_major_scale(chroma: &[f32; 12]) -> bool {
    // Check for presence of major 3rd interval (4 semitones) or minor 3rd (3 semitones)
    // Major scale has stronger major 3rd
    let major_third_strength = chroma[4]; // Assume C major, major third is E
    let minor_third_strength = chroma[3]; // Minor third is Eb

    major_third_strength > minor_third_strength
}

/// Energy feature: combination of RMS and spectral brightness
fn compute_energy(rms_val: f32, spectral_centroid: f32) -> f64 {
    let rms_component = (rms_val * 10.0).min(1.0) as f64;
    let sc_component = (spectral_centroid / 5000.0).min(1.0) as f64;

    (rms_component * 0.6 + sc_component * 0.4).min(1.0)
}

/// Valence feature: perceived happiness/positivity
/// Factors: faster tempo, major key, bright sound, higher energy
fn compute_valence(tempo: f64, is_major: bool, spectral_centroid: f32, energy: f64) -> f64 {
    let tempo_factor = ((tempo / 180.0).min(1.0)) * 0.3;
    let key_factor = if is_major { 0.3 } else { 0.1 };
    let brightness_factor = ((spectral_centroid / 4000.0).min(1.0)) as f64 * 0.2;
    let energy_factor = energy * 0.2;

    (tempo_factor + key_factor + brightness_factor + energy_factor).min(1.0)
}

/// Danceability feature: perceived suitability for dancing
/// Factors: steady tempo in dance range (100-140 BPM), regular beat
fn compute_danceability(tempo: f64, zcr: f32) -> f64 {
    // Peak danceability at 120 BPM
    let tempo_factor = (1.0 - ((tempo - 120.0) / 60.0).abs().min(1.0)) * 0.7;
    
    // Regular beat (lower ZCR variation) contributes to danceability
    let beat_regularity = (1.0 - (zcr * 2.0).min(1.0)) as f64 * 0.3;

    (tempo_factor + beat_regularity).min(1.0)
}

/// Acousticness feature: perceived acoustic vs electronic quality
/// Lower spectral centroid = more acoustic
fn compute_acousticness(spectral_centroid: f32) -> f64 {
    ((1.0 - spectral_centroid / 4000.0).max(0.0)) as f64
}

/// Speechiness feature: presence of spoken words
/// Based on zero crossing rate (speech has high ZCR)
fn compute_speechiness(zcr: f32) -> f64 {
    ((zcr * 5.0).min(1.0)) as f64
}

/// Instrumentalness feature: absence of vocals
/// Inverse of speechiness
fn compute_instrumentalness(speechiness: f64) -> f64 {
    (1.0 - speechiness * 2.0).max(0.0).min(1.0)
}

/// Liveness feature: perceived presence/live quality
/// Based on dynamic range in the signal
fn compute_liveness(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.5;
    }

    // Compute dynamic range
    let max_abs = samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    let sorted_abs: Vec<f32> = {
        let mut v = samples.iter().map(|x| x.abs()).collect::<Vec<_>>();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    };

    // Quiet level = bottom 10th percentile
    let quiet_level = sorted_abs.get(sorted_abs.len() / 10).unwrap_or(&0.0);

    // Dynamic range = ratio of max to min
    let dynamic_range = if *quiet_level > 0.0 {
        (max_abs / quiet_level).log10() / 4.0 // Normalize to roughly 0-1
    } else {
        0.5
    };

    dynamic_range.clamp(0.0, 1.0) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tempo() {
        // Create a synthetic signal with known frequency
        let sample_rate = 44100;
        let frequency = 2.0; // 2 Hz = 120 BPM (60 beats per second * 2)
        let samples: Vec<f32> = (0..sample_rate * 2)
            .map(|i| ((i as f32 / sample_rate as f32) * frequency * std::f32::consts::TAU).sin())
            .collect();

        let tempo = estimate_tempo(&samples, sample_rate as u32);
        assert!(tempo > 100.0 && tempo < 140.0); // Should be around 120 BPM
    }

    #[test]
    fn test_compute_energy() {
        let energy = compute_energy(0.5, 2000.0);
        assert!(energy >= 0.0 && energy <= 1.0);
    }

    #[test]
    fn test_compute_valence() {
        let valence = compute_valence(120.0, true, 3000.0, 0.5);
        assert!(valence >= 0.0 && valence <= 1.0);
        assert!(valence > 0.3); // Should be reasonably high (major, bright, good tempo)
    }
}
