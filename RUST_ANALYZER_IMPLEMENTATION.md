# Rust-based Audio Analyzer Implementation

## Overview

This document describes the new Rust-based audio feature extraction system implemented for Vibe-On. The system replaces the Python/Essentia analyzer with a fully native Rust implementation while maintaining backward compatibility with optional Python/Essentia fallback.

## Architecture

### High-Level Flow

```
┌─────────────────────────────────────┐
│  Frontend: analyze_track() command  │
└────────────┬────────────────────────┘
             │
             ▼
┌─────────────────────────────────────┐
│  analyzer.rs: AudioAnalyzer         │
│  ├─ Tries Rust analyzer first       │
│  └─ Falls back to Python/Essentia   │
└────────┬────────────────┬───────────┘
         │                │
    ┌────▼────────┐  ┌───▼──────────────┐
    │ Rust Path   │  │ Python Path      │
    │ (always     │  │ (optional,       │
    │  available) │  │  dev/advanced)   │
    └────┬────────┘  └─────────────────┘
         │
         ▼
    ┌─────────────────────────────────┐
    │ 1. audio_loader.rs              │
    │    Load & decode audio file     │
    │    (symphonia)                  │
    └────┬────────────────────────────┘
         │
         ▼
    ┌─────────────────────────────────┐
    │ 2. dsp.rs                       │
    │    Extract DSP features:        │
    │    - RMS, ZCR                   │
    │    - Spectral centroid          │
    │    - Chroma features            │
    │    (rustfft)                    │
    └────┬────────────────────────────┘
         │
         ▼
    ┌─────────────────────────────────┐
    │ 3. rust_analyzer.rs             │
    │    Compute mood features:       │
    │    - Tempo, Key, Energy         │
    │    - Valence, Danceability      │
    │    - Heuristics                 │
    └────┬────────────────────────────┘
         │
         ▼
    ┌─────────────────────────────────┐
    │ AudioFeatures (cached in DB)    │
    │ analysis_backend: "rust"        │
    └─────────────────────────────────┘
```

## Module Structure

### 1. **audio_loader.rs** - Audio Decoding
**Purpose**: Load audio files in any format and convert to mono f32 PCM

**Key Functions**:
- `load_audio(path: &str)` - Loads entire file
- `load_audio_truncated(path, max_duration)` - Loads subset for performance
- `convert_to_mono_f32(decoded)` - Handles format-specific conversion

**Supported Formats**: WAV, MP3, FLAC, OGG, AAC (via symphonia's codec support)

**Implementation Details**:
- Uses symphonia for format detection and decoding
- Converts all formats to mono f32 samples
- Handles multi-channel audio by averaging channels
- Normalizes output to approximately [-1.0, 1.0] range

### 2. **dsp.rs** - Digital Signal Processing
**Purpose**: Extract low-level audio features using DSP algorithms

**Key Features**:
| Feature | Implementation | Use Case |
|---------|------------------|----------|
| RMS | `sqrt(mean(x²))` | Energy level |
| ZCR | Sign changes/frame | Speech detection |
| Spectral Centroid | FFT-based weighted average | Brightness |
| Spectral Rolloff | FFT cumulative energy | Frequency range |
| Chroma Features | Pitch class distribution (12 bins) | Key/scale detection |
| Dominant Frequency | FFT peak finding | Pitch detection |

**Algorithms**:
- **FFT**: Uses `rustfft` for spectral analysis
- **Framing**: Processes full signal (no windowing for simplicity)
- **Normalization**: All features normalized to 0-1 range

### 3. **rust_analyzer.rs** - Mood Feature Extraction
**Purpose**: Convert DSP features to high-level mood features

**Mood Features Computed**:

```rust
pub struct AudioFeatures {
    valence: f64,           // 0=sad, 1=happy (based on tempo, key, brightness, energy)
    energy: f64,            // 0=low, 1=high (RMS + spectral centroid)
    danceability: f64,      // 0=not danceable, 1=very danceable (tempo + beat regularity)
    tempo: f64,             // BPM (beat detection via energy peaks)
    key: i32,               // 0-11 (C to B, from strongest chroma bin)
    loudness: f64,          // dB (20*log10(RMS))
    instrumentalness: f64,  // 0=vocal, 1=instrumental (inverse of speechiness)
    acousticness: f64,      // 0=electronic, 1=acoustic (inverse of spectral centroid)
    speechiness: f64,       // 0=music, 1=speech (based on ZCR)
    liveness: f64,          // 0=studio, 1=live (dynamic range)
    analysis_version: i32,  // 2 = Rust analyzer
    analysis_backend: Option<String>, // "rust" or "essentia"
}
```

**Heuristics Used**:

1. **Energy**: Weighted combination of RMS and spectral brightness
   ```
   energy = (RMS * 10) * 0.6 + (SC / 5000) * 0.4
   ```

2. **Valence**: Combination of tempo, key, brightness, and energy
   ```
   valence = (tempo/180)*0.3 + (is_major?1.0:0.3)*0.3 + (SC/4000)*0.2 + energy*0.2
   ```

3. **Danceability**: Tempo proximity to 120 BPM + beat regularity
   ```
   danceability = (1 - |tempo-120|/60)*0.7 + beat_regularity*0.3
   ```

4. **Tempo**: Peak detection in frame-based energy contour
   - Computes energy for 100ms frames
   - Detects peaks (local maxima)
   - Converts peak distance to BPM (60 beats/second / distance)
   - Clamps to 60-200 BPM range

5. **Key**: Strongest bin in chroma vector (0-11 scale)

6. **Instrumentalness**: `max(0, 1 - speechiness * 2)`

7. **Acousticness**: `max(0, 1 - spectral_centroid / 4000)`

8. **Speechiness**: `min(1, ZCR * 5)` (speech has high zero-crossing rate)

9. **Liveness**: Dynamic range computed from signal
   ```
   dynamic_range = log10(max_abs / quiet_level) / 4
   ```

### 4. **analyzer.rs** - Orchestration & Fallback
**Purpose**: Coordinate analysis with Rust-first/Python-fallback strategy

**Strategy**:
1. **Try Rust Analyzer First** (always available)
   - Fast (single-threaded, no subprocess)
   - Works offline
   - No external dependencies
   - Consistent results

2. **Fall Back to Python/Essentia** (if available)
   - Check for Python 3.11
   - Check for Essentia installation
   - Run sidecar script if both available
   - Marked with `analysis_backend: "essentia"`

3. **Database Caching**
   - Results cached by file path
   - `analysis_version` field used for cache invalidation
   - `analysis_backend` field indicates which analyzer was used

**New Methods**:
- `analyze_track_python()` - Private method for Python execution
- `check_availability()` - Checks if Python/Essentia are available
- `new_prefer_python()` - Constructor to prefer Python (for testing)

### 5. **types.rs** - Data Structures
**New Field**: `analysis_backend: Option<String>`
- `None` or `Some("rust")` - Rust analyzer
- `Some("essentia")` - Python/Essentia analyzer

### 6. **schema.rs** - Database Schema
**New Column**: `analysis_backend TEXT`
- Tracks which backend was used for analysis
- Useful for debugging and statistics

### 7. **db.rs** - Database Operations
**Updated Methods**:
- `upsert_features()` - Now saves `analysis_backend`
- `get_features()` - Now retrieves `analysis_backend`

## Performance Characteristics

### Rust Analyzer
- **Single track**: ~200-500ms (depends on file size and CPU)
- **Memory**: ~200MB for typical 3-4 minute song
- **CPU**: Single-threaded, but CPU-intensive (FFT operations)
- **Network**: None
- **Dependencies**: Compiled into binary

### Python/Essentia Analyzer
- **Single track**: ~500ms-2s (subprocess overhead + ML models)
- **Memory**: Higher (Python + Essentia libraries)
- **CPU**: Multi-threaded (Essentia uses BLAS)
- **Network**: None
- **Dependencies**: External (Python 3.11, Essentia library)

### Caching
- First analysis: Rust (~200-500ms)
- Cached reads: <1ms (database lookup)
- Cache invalidation: Manual via `analysis_version`

## Development & Testing

### Building
```bash
cd src-tauri
cargo build --release
```

### Testing
```bash
cargo test --lib mood
```

### Manual Testing
```rust
// In your code:
let (features, metadata) = rust_analyzer::analyze_audio_file_rust("path/to/song.mp3")?;
println!("Valence: {}", features.valence);
```

### Comparison with Python
To compare Rust vs Python output on the same file:
1. Analyze with Rust (automatic)
2. Clear cache: `DELETE FROM audio_features WHERE track_path = ?`
3. Set `prefer_python = true` in analyzer
4. Analyze with Python
5. Compare `analysis_backend` and feature values

## Integration with Frontend

### No Changes Required to TypeScript
The frontend code remains unchanged:
- `MOOD_PRESETS` still use same value ranges (0-1)
- `AudioFeatures` interface matches exactly
- Cache system works transparently
- Mood radio generation logic unchanged

### New Field for UI (Optional)
Display which backend was used:
```typescript
<div>
  {track.features?.analysis_backend === 'rust' && '🦀 Native Analysis'}
  {track.features?.analysis_backend === 'essentia' && '🐍 ML Analysis'}
</div>
```

## Fallback to Python/Essentia

### When to Use Python
1. **Higher Accuracy Needed**: ML-based models are more accurate
2. **Specific Research**: Need exact Essentia algorithms
3. **Development/Testing**: Benchmark Rust vs Python
4. **Advanced Analysis**: Custom Essentia extractors

### How to Force Python
```rust
// In commands.rs setup:
let analyzer = AudioAnalyzer::new_prefer_python(&resources_dir);
```

### Checking Availability
```rust
let status = analyzer.check_availability();
if status.available {
    println!("Python {} + Essentia {} available", 
             status.python_version, 
             status.essentia_version);
}
```

## Dependencies

### New Cargo Dependencies
```toml
symphonia = { version = "0.5", features = ["default"] }  # Audio decoding
rustfft = "6.2"                                           # FFT
num-complex = "0.4"                                       # Complex numbers
chrono = "0.4"                                            # Timestamps
```

### No External Runtime Dependencies
- All included in Tauri binary
- No Python or Essentia required at runtime
- Cross-platform compatible (macOS, Windows, Linux)

## Known Limitations

### Rust Analyzer Heuristics
1. **Valence/Energy Approximations**: Cannot match ML models perfectly
   - Python uses trained models, Rust uses heuristics
   - Differences expected in edge cases
   - Results are consistent and usable for mood-based queuing

2. **No Vocal/Instrument Detection**: Cannot extract vocal/instrumental split
   - Essentia uses ML models
   - Rust approximates via ZCR (rough estimate)

3. **BPM Detection**: Simplified tempo estimation
   - Works well for regular beats
   - May struggle with complex/polyrhythmic music
   - Falls back to 120 BPM if detection fails

4. **Key Detection**: Based on chroma, no minor/major distinction
   - Simplified scale detection
   - Works for standard tonal music
   - May be inaccurate for atonal/dissonant music

## Future Improvements

1. **Onset Detection**: Detect beats for better BPM
2. **Spectral Flux**: Track timbre changes
3. **MFCC Features**: Cepstral coefficients for better energy/brightness
4. **Harmonic/Percussive Separation**: HPSS algorithm
5. **Multi-threaded FFT**: Parallel processing for speed
6. **WebAssembly Port**: Analyze on client-side (future)

## Debugging

### Enable Debug Logging
```rust
println!("[Rust Analyzer] Starting analysis of: {}", path);
println!("[Audio Loader] Loaded {} samples", samples.len());
println!("[DSP] Spectral centroid: {}", sc);
```

### Troubleshooting

**Problem**: Compilation errors with symphonia
- **Solution**: Ensure `symphonia = "0.5"` in Cargo.toml with `default` features

**Problem**: Audio files not loading
- **Solution**: Check file format is supported (WAV, MP3, FLAC, OGG)
- **Solution**: Verify file is valid and not corrupted

**Problem**: Rust analysis much faster than Python
- **Solution**: This is expected! Rust has no subprocess overhead
- **Solution**: Use Python/Essentia if you need ML-enhanced accuracy

**Problem**: Very different feature values between Rust and Python
- **Solution**: Expected for valence/energy (heuristics vs ML)
- **Solution**: Tempo/key may differ in complex music
- **Solution**: Check both use same audio file

## References

- [Symphonia Documentation](https://github.com/pdeljanov/Symphonia)
- [RustFFT](https://github.com/ejmahler/RustFFT)
- [Essentia Audio Analysis](https://essentia.upf.edu/)
- [Audio Feature Extraction](https://en.wikipedia.org/wiki/Feature_extraction#Audio_signals)
