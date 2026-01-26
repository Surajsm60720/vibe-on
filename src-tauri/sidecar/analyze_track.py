#!/usr/bin/env python3.11
"""
Essentia Audio Feature Extractor for Vibe-On
Outputs JSON with audio features to stdout.

Exit codes:
  0 - Success
  1 - File not found
  2 - Unsupported format
  3 - Analysis error
  4 - Essentia not available
"""

import json
import sys
import os

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No audio file path provided"}))
        sys.exit(3)

    audio_path = sys.argv[1]

    # Verify file exists
    if not os.path.exists(audio_path):
        print(json.dumps({"error": f"File not found: {audio_path}"}))
        sys.exit(1)

    try:
        import essentia
        import essentia.standard as es
    except ImportError as e:
        print(json.dumps({"error": f"Essentia not installed: {e}"}))
        sys.exit(4)

    try:
        # Load audio
        loader = es.MonoLoader(filename=audio_path)
        audio = loader()

        # Initialize algorithms
        rhythm_extractor = es.RhythmExtractor2013(method="multifeature")
        key_extractor = es.KeyExtractor()
        loudness = es.Loudness()
        dynamic_complexity = es.DynamicComplexity()
        
        # For valence/energy approximation, we use spectral features
        # Note: True valence requires trained ML models, this is an approximation
        spectral_centroid = es.SpectralCentroidTime()
        zero_crossing = es.ZeroCrossingRate()
        rms = es.RMS()
        
        # Compute features
        bpm, beats, beats_confidence, _, beats_intervals = rhythm_extractor(audio)
        key, scale, key_strength = key_extractor(audio)
        loud = loudness(audio)
        
        # Compute spectral features for mood approximation
        sc = spectral_centroid(audio)
        zcr = zero_crossing(audio)
        rms_val = rms(audio)
        
        # Key mapping (Essentia returns string like "C", "C#", etc.)
        key_map = {
            "C": 0, "C#": 1, "Db": 1, "D": 2, "D#": 3, "Eb": 3,
            "E": 4, "F": 5, "F#": 6, "Gb": 6, "G": 7, "G#": 8,
            "Ab": 8, "A": 9, "A#": 10, "Bb": 10, "B": 11
        }
        key_num = key_map.get(key, -1)

        # Approximate danceability from rhythm regularity and tempo
        # Higher BPM in dance range (100-140) + steady beats = more danceable
        dance_tempo_factor = max(0, 1 - abs(bpm - 120) / 60)  # Peak at 120 BPM
        danceability = min(1.0, dance_tempo_factor * beats_confidence)

        # Energy approximation from RMS and spectral centroid
        # Normalize to 0-1 range
        energy = min(1.0, max(0.0, (rms_val * 10 + sc / 5000) / 2))

        # Valence approximation (very rough - ideally needs ML model)
        # Higher tempo + major key + bright sound = happier
        is_major = scale.lower() == "major"
        valence = min(1.0, max(0.0,
            0.3 * (bpm / 180) +  # Faster = happier
            0.3 * (1.0 if is_major else 0.3) +  # Major = happier
            0.2 * (sc / 4000) +  # Brighter = happier
            0.2 * energy  # More energy = happier perception
        ))

        # Speechiness from zero crossing rate (speech has more ZCR)
        speechiness = min(1.0, zcr * 5)  # Scale up, cap at 1

        # Instrumentalness - inverse of ZCR (vocals have high ZCR)
        instrumentalness = max(0.0, 1.0 - speechiness * 2)

        # Liveness - approximated by dynamic range
        dyn_complexity, loudness_range = dynamic_complexity(audio)
        liveness = min(1.0, loudness_range / 20)  # Higher dynamic range = more live

        # Acousticness - estimated from spectral features (lower SC = more acoustic)
        acousticness = max(0.0, 1.0 - sc / 4000)

        result = {
            "valence": round(valence, 4),
            "energy": round(energy, 4),
            "danceability": round(danceability, 4),
            "tempo": round(bpm, 2),
            "key": key_num,
            "loudness": round(loud, 2),
            "instrumentalness": round(instrumentalness, 4),
            "acousticness": round(acousticness, 4),
            "speechiness": round(speechiness, 4),
            "liveness": round(liveness, 4),
            "analysis_version": 1
        }

        print(json.dumps(result))
        sys.exit(0)

    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(3)


if __name__ == "__main__":
    main()
