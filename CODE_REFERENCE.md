# Code Reference - Key Implementation Details

## 1. How the Fallback System Works

### analyzer.rs - Main Entry Point
```rust
pub fn analyze_track(&self, audio_path: &str) -> Result<AudioFeatures, String> {
    // Step 1: Try Rust analyzer (always works)
    println!("[Analyzer] Attempting Rust analysis for: {}", audio_path);
    match rust_analyzer::analyze_audio_file_rust(audio_path) {
        Ok(mut features) => {
            println!("[Analyzer] Rust analysis succeeded");
            features.analysis_backend = Some("rust".to_string());
            return Ok(features);
        }
        Err(rust_err) => {
            println!("[Analyzer] Rust analysis failed: {}", rust_err);
            if !self.prefer_python {
                return Err(rust_err);  // Use Rust error
            }
            // Otherwise continue to Python fallback...
        }
    }

    // Step 2: Try Python/Essentia as fallback
    if self.prefer_python || self.check_availability().available {
        println!("[Analyzer] Attempting Python/Essentia analysis");
        match self.analyze_track_python(audio_path) {
            Ok(mut features) => {
                println!("[Analyzer] Python analysis succeeded");
                features.analysis_backend = Some("essentia".to_string());
                return Ok(features);
            }
            Err(python_err) => {
                println!("[Analyzer] Python analysis failed: {}", python_err);
                return Err(python_err);
            }
        }
    }

    // Step 3: Both failed
    Err("No analyzer available - Rust failed and Python/Essentia not available".to_string())
}
```

## 2. Audio Decoding with Symphonia

### audio_loader.rs - Format Handling
```rust
/// Convert AudioBufferRef to mono f32 samples
fn convert_to_mono_f32(decoded: &AudioBufferRef) -> Vec<f32> {
    let num_frames = decoded.frames();
    let num_channels = decoded.spec().channels.count();
    let mut mono_samples = Vec::with_capacity(num_frames);

    match decoded {
        // Handle different sample formats
        AudioBufferRef::F32(buf) => {
            for frame_idx in 0..num_frames {
                let mut sample_sum = 0.0f32;
                for ch_idx in 0..num_channels {
                    let channel_data = buf.chan(ch_idx);
                    sample_sum += channel_data[frame_idx];
                }
                mono_samples.push(sample_sum / num_channels as f32);
            }
        }
        AudioBufferRef::S16(buf) => {
            // Same pattern for 16-bit signed samples
            // Convert to f32 during averaging
        }
        AudioBufferRef::S32(buf) => { /* ... */ }
        AudioBufferRef::F64(buf) => { /* ... */ }
        AudioBufferRef::U8(buf) => { /* ... */ }
        _ => eprintln!("Unsupported audio format"),
    }

    mono_samples
}
```

## 3. DSP Feature Extraction

### dsp.rs - Low-Level Features
```rust
/// Compute RMS (Root Mean Square) energy
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Compute Zero Crossing Rate (speech detection)
pub fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 { return 0.0; }
    let mut crossings = 0;
    for i in 1..samples.len() {
        if samples[i - 1] * samples[i] < 0.0 {
            crossings += 1;
        }
    }
    crossings as f32 / samples.len() as f32
}

/// Compute Spectral Centroid via FFT (brightness indicator)
pub fn spectral_centroid(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    
    let fft_size = samples.len().next_power_of_two();
    let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(fft_size);

    for &sample in samples.iter() {
        buffer.push(Complex { re: sample, im: 0.0 });
    }
    for _ in samples.len()..fft_size {
        buffer.push(Complex { re: 0.0, im: 0.0 });
    }

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    // Calculate: sum(bin_index * magnitude) / sum(magnitude)
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for (i, c) in buffer.iter().enumerate() {
        let mag = c.norm();
        numerator += i as f32 * mag;
        denominator += mag;
    }

    if denominator == 0.0 { 0.0 } else { numerator / denominator }
}

/// Compute Chroma Features (12 pitch classes for key detection)
pub fn chroma_features(samples: &[f32], sample_rate: f32) -> [f32; 12] {
    let mut chroma = [0.0; 12];
    // ... FFT and frequency-to-pitch mapping ...
    chroma
}
```

## 4. Mood Feature Computation

### rust_analyzer.rs - Feature Heuristics
```rust
/// Estimate tempo using frame-based energy detection
fn estimate_tempo(samples: &[f32], sample_rate: u32) -> f64 {
    let frame_size = sample_rate as usize / 10;  // 100ms frames
    if frame_size < 2 { return 120.0; }

    let mut frame_energies = Vec::new();
    for chunk in samples.chunks(frame_size) {
        let energy = rms(chunk);
        frame_energies.push(energy);
    }

    // Detect peaks (local maxima)
    let mut peak_distances = Vec::new();
    let mut last_peak = None;

    for i in 1..frame_energies.len() - 1 {
        if frame_energies[i] > frame_energies[i - 1] && 
           frame_energies[i] > frame_energies[i + 1] {
            if let Some(prev) = last_peak {
                peak_distances.push(i - prev);
            }
            last_peak = Some(i);
        }
    }

    if peak_distances.is_empty() { return 120.0; }

    // Convert peak distance to BPM
    let avg_peak_distance = peak_distances.iter().sum::<usize>() as f64 / 
                           peak_distances.len() as f64;
    let beat_duration_seconds = avg_peak_distance * frame_size as f64 / 
                                sample_rate as f64;
    let bpm = (60.0 / beat_duration_seconds).clamp(60.0, 200.0);

    bpm
}

/// Detect musical key from chroma vector
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

/// Calculate Valence (perceived happiness) heuristic
fn compute_valence(tempo: f64, is_major: bool, spectral_centroid: f32, 
                   energy: f64) -> f64 {
    let tempo_factor = ((tempo / 180.0).min(1.0)) * 0.3;
    let key_factor = if is_major { 0.3 } else { 0.1 };
    let brightness_factor = ((spectral_centroid / 4000.0).min(1.0)) as f64 * 0.2;
    let energy_factor = energy * 0.2;

    (tempo_factor + key_factor + brightness_factor + energy_factor).min(1.0)
}

/// Calculate Danceability heuristic
fn compute_danceability(tempo: f64, zcr: f32) -> f64 {
    let tempo_factor = (1.0 - ((tempo - 120.0) / 60.0).abs().min(1.0)) * 0.7;
    let beat_regularity = (1.0 - (zcr * 2.0).min(1.0)) as f64 * 0.3;

    (tempo_factor + beat_regularity).min(1.0)
}
```

## 5. Database Integration

### schema.rs - Table Definition
```rust
pub const AUDIO_FEATURES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS audio_features (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_path TEXT NOT NULL UNIQUE,
    valence REAL,
    energy REAL,
    danceability REAL,
    tempo REAL,
    key INTEGER,
    loudness REAL,
    instrumentalness REAL,
    acousticness REAL,
    speechiness REAL,
    liveness REAL,
    analysis_version INTEGER DEFAULT 1,
    analyzed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    analysis_error TEXT,
    analysis_backend TEXT              -- NEW: Tracks which analyzer was used
);

CREATE INDEX IF NOT EXISTS idx_audio_features_path ON audio_features(track_path);
"#;
```

### db.rs - Query with Backend Field
```rust
pub fn upsert_features(&self, track_path: &str, features: &AudioFeatures) 
    -> Result<()> 
{
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO audio_features (
            track_path, valence, energy, danceability, tempo, key, loudness, 
            instrumentalness, acousticness, speechiness, liveness, 
            analysis_version, analysis_error, analysis_backend
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(track_path) DO UPDATE SET
            valence = excluded.valence,
            energy = excluded.energy,
            -- ... other fields ...
            analysis_backend = excluded.analysis_backend",
        params![
            track_path,
            features.valence,
            features.energy,
            features.danceability,
            features.tempo,
            features.key,
            features.loudness,
            features.instrumentalness,
            features.acousticness,
            features.speechiness,
            features.liveness,
            features.analysis_version,
            features.analysis_error,
            &features.analysis_backend,  // NEW
        ],
    )?;
    Ok(())
}
```

## 6. Types - AudioFeatures Structure

### types.rs - New Field Added
```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AudioFeatures {
    pub valence: f64,
    pub energy: f64,
    pub danceability: f64,
    pub tempo: f64,
    pub key: i32,
    pub loudness: f64,
    pub instrumentalness: f64,
    pub acousticness: f64,
    pub speechiness: f64,
    pub liveness: f64,
    pub analysis_version: i32,
    pub analyzed_at: Option<String>,
    pub analysis_error: Option<String>,
    pub analysis_backend: Option<String>,  // NEW: "rust" or "essentia"
}
```

## 7. Usage Examples

### From Commands
```rust
#[tauri::command]
pub async fn analyze_track(
    path: String,
    state: State<'_, MoodState>,
) -> Result<AudioFeatures, String> {
    let analyzer = state.analyzer.as_ref()
        .ok_or("Analyzer not initialized")?;

    let db = state.db.as_ref()
        .ok_or("Database not initialized")?;

    // Check cache first
    if let Ok(Some(features)) = db.get_features(&path) {
        if features.analysis_error.is_none() {
            return Ok(features);  // Cached result
        }
    }

    // Run analysis
    match analyzer.analyze_track(&path) {
        Ok(features) => {
            db.upsert_features(&path, &features)?;  // Cache it
            Ok(features)
        }
        Err(e) => {
            db.mark_error(&path, &e)?;  // Remember the error
            Err(e)
        }
    }
}
```

### From Frontend (unchanged!)
```typescript
// TypeScript remains the same
const features = await invoke('analyze_track', { path: audioPath });

console.log(`Valence: ${features.valence}`);
console.log(`Analyzed with: ${features.analysis_backend}`);  // "rust" or "essentia"

// Mood presets work exactly as before
const radioQueue = tracks.filter(t => {
    return t.features.valence > 0.6 && t.features.energy > 0.5;  // happy preset
});
```

## 8. Feature Value Ranges

All features normalized to [0, 1] scale:

```
Valence:           0.0 (sad) ←→ 1.0 (happy)
Energy:            0.0 (low) ←→ 1.0 (high)
Danceability:      0.0 (not) ←→ 1.0 (very)
Instrumentalness:  0.0 (vocal) ←→ 1.0 (instrumental)
Acousticness:      0.0 (electronic) ←→ 1.0 (acoustic)
Speechiness:       0.0 (music) ←→ 1.0 (speech)
Liveness:          0.0 (studio) ←→ 1.0 (live)

Tempo:             60-200 BPM
Key:               0-11 (C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
Loudness:          -60 to 0 dB
Analysis Version:  2 (Rust) or 1 (Python)
```

---

**These code snippets show the key implementation details for understanding how the system works.**
