# Essentia Audio Analyzer Sidecar

Python sidecar for audio feature extraction using [Essentia](https://essentia.upf.edu/).

> **Important**: Essentia requires Python 3.11 (not 3.12+). Python 3.14/3.13/3.12 are NOT supported due to deprecated `imp` module.

## Installation

### macOS (Apple Silicon / arm64)
```bash
# Install Python 3.11
brew install python@3.11

# Install Essentia with pip3.11
pip3.11 install --user essentia
```

### Linux
```bash
pip install essentia
```

### Windows
```bash
pip install essentia
```

## Usage

```bash
python analyze_track.py /path/to/audio.mp3
```

Outputs JSON to stdout:
```json
{
  "valence": 0.72,
  "energy": 0.85,
  "danceability": 0.68,
  "tempo": 128.5,
  "key": 7,
  "loudness": -5.2,
  "instrumentalness": 0.12,
  "acousticness": 0.23,
  "speechiness": 0.05,
  "liveness": 0.15,
  "analysis_version": 1
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | File not found |
| 2 | Unsupported format |
| 3 | Analysis error |
| 4 | Essentia not available |

## Supported Formats

- MP3, FLAC, WAV, OGG, M4A, AAC
- Any format supported by FFmpeg (if Essentia built with FFmpeg support)

## Feature Descriptions

| Feature | Range | Description |
|---------|-------|-------------|
| valence | 0-1 | Musical positivity (sad → happy) |
| energy | 0-1 | Perceptual intensity |
| danceability | 0-1 | Suitability for dancing |
| tempo | BPM | Beats per minute |
| key | 0-11 | Musical key (C=0 to B=11) |
| loudness | dB | Overall loudness |
| instrumentalness | 0-1 | Likelihood of no vocals |
| acousticness | 0-1 | Acoustic vs electronic |
| speechiness | 0-1 | Presence of spoken words |
| liveness | 0-1 | Presence of audience/live feel |

## Troubleshooting

### "Essentia not installed"
Run: `pip install essentia`

### arm64 Mac issues
Use Rosetta: `arch -x86_64 pip install essentia`

### Analysis fails on specific file
Check if file is corrupted or in an unsupported format.
