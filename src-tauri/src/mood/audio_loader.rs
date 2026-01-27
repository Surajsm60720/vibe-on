/// Audio File Loader using Symphonia
/// Supports WAV, MP3, FLAC, OGG, and other formats

use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub sample_rate: u32,
    pub channels: u8,
    pub duration_samples: u64,
}

impl AudioMetadata {
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.duration_samples as f64 / self.sample_rate as f64
        }
    }
}

/// Convert AudioBufferRef to mono f32 samples
fn convert_to_mono_f32(decoded: &AudioBufferRef) -> Vec<f32> {
    let num_frames = decoded.frames();
    let num_channels = decoded.spec().channels.count();
    let mut mono_samples = Vec::with_capacity(num_frames);

    match decoded {
        AudioBufferRef::F32(buf) => {
            for frame_idx in 0..num_frames {
                let mut sample_sum = 0.0f32;

                for ch_idx in 0..num_channels {
                    let channel_data = buf.chan(ch_idx);
                    sample_sum += channel_data[frame_idx];
                }

                let mono_sample = sample_sum / num_channels as f32;
                mono_samples.push(mono_sample);
            }
        }
        AudioBufferRef::S16(buf) => {
            for frame_idx in 0..num_frames {
                let mut sample_sum = 0.0f32;

                for ch_idx in 0..num_channels {
                    let channel_data = buf.chan(ch_idx);
                    sample_sum += channel_data[frame_idx] as f32;
                }

                let mono_sample = sample_sum / num_channels as f32;
                mono_samples.push(mono_sample);
            }
        }
        AudioBufferRef::S32(buf) => {
            for frame_idx in 0..num_frames {
                let mut sample_sum = 0.0f32;

                for ch_idx in 0..num_channels {
                    let channel_data = buf.chan(ch_idx);
                    sample_sum += channel_data[frame_idx] as f32;
                }

                let mono_sample = sample_sum / num_channels as f32;
                mono_samples.push(mono_sample);
            }
        }
        AudioBufferRef::F64(buf) => {
            for frame_idx in 0..num_frames {
                let mut sample_sum = 0.0f64;

                for ch_idx in 0..num_channels {
                    let channel_data = buf.chan(ch_idx);
                    sample_sum += channel_data[frame_idx];
                }

                let mono_sample = (sample_sum / num_channels as f64) as f32;
                mono_samples.push(mono_sample);
            }
        }
        AudioBufferRef::U8(buf) => {
            for frame_idx in 0..num_frames {
                let mut sample_sum = 0.0f32;

                for ch_idx in 0..num_channels {
                    let channel_data = buf.chan(ch_idx);
                    // U8 is unsigned, normalize from 0-255 to -1.0 to 1.0
                    sample_sum += ((channel_data[frame_idx] as f32) / 128.0) - 1.0;
                }

                let mono_sample = sample_sum / num_channels as f32;
                mono_samples.push(mono_sample);
            }
        }
        _ => {
            eprintln!("[Audio Loader] Unsupported audio format");
        }
    }

    mono_samples
}

/// Load audio file and convert to mono f32 samples
/// Returns (samples, metadata)
pub fn load_audio(path: &str) -> Result<(Vec<f32>, AudioMetadata), String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open audio file: {}", e))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create hint from file extension
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension() {
        if let Some(ext_str) = ext.to_str() {
            hint.with_extension(ext_str);
        }
    }

    let probed = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &Default::default(),
        )
        .map_err(|e| format!("Failed to probe audio format: {}", e))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .first()
        .ok_or("No audio tracks found in file")?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track.codec_params.channels.map(|ch| ch.count() as u8).unwrap_or(2);

    let mut decoder = get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut samples = Vec::new();
    let mut total_frames = 0u64;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(e) => return Err(format!("Error reading packet: {}", e)),
        };

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("Failed to decode packet: {}", e))?;

        total_frames += decoded.frames() as u64;

        // Convert to mono using format-specific conversion
        let mono = convert_to_mono_f32(&decoded);
        samples.extend(mono);
    }

    let metadata = AudioMetadata {
        sample_rate,
        channels,
        duration_samples: total_frames,
    };

    // Normalize samples to approximately [-1.0, 1.0] range
    if !samples.is_empty() {
        let max_abs = samples
            .iter()
            .map(|x: &f32| x.abs())
            .fold(0.0f32, f32::max);

        if max_abs > 1.0 {
            let scale = 1.0 / max_abs;
            for sample in &mut samples {
                *sample *= scale;
            }
        }
    }

    println!(
        "[Audio Loader] Loaded {} samples at {} Hz, {} channels",
        samples.len(),
        sample_rate,
        channels
    );

    Ok((samples, metadata))
}

/// Load only a subset of the audio file (useful for large files)
pub fn load_audio_truncated(
    path: &str,
    max_duration_seconds: Option<f64>,
) -> Result<(Vec<f32>, AudioMetadata), String> {
    let (mut samples, metadata) = load_audio(path)?;

    if let Some(max_dur) = max_duration_seconds {
        let max_samples = (max_dur * metadata.sample_rate as f64) as usize;
        if samples.len() > max_samples {
            samples.truncate(max_samples);
        }
    }

    Ok((samples, metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_duration() {
        let meta = AudioMetadata {
            sample_rate: 44100,
            channels: 2,
            duration_samples: 44100,
        };
        assert!((meta.duration_seconds() - 1.0).abs() < 0.01);
    }
}
