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
    # Argument Handling
    # Check if we should default to Librosa (Windows) or if Essentia is available
    essentia_available = False
    if sys.platform != "win32": # On Windows, skip Essentia as requested
        try:
            import essentia
            import essentia.standard as es
            essentia_available = True
        except ImportError:
            pass

    if len(sys.argv) > 1 and sys.argv[1] == "--check":
        print(json.dumps({
            "available": True,
            "engine": "essentia" if essentia_available else "librosa",
            "essentia_version":  essentia.__version__ if essentia_available else None
        }))
        sys.exit(0)

    if len(sys.argv) < 2:
        print(json.dumps({"error": "No audio file path provided"}))
        sys.exit(3)

    audio_path = sys.argv[1]

    # Verify file exists
    if not os.path.exists(audio_path):
        print(json.dumps({"error": f"File not found: {audio_path}"}))
        sys.exit(1)

    # Validation: If neither is available
    if not essentia_available:
        try:
            import librosa
            import numpy as np
        except ImportError:
             print(json.dumps({"error": "Neither Essentia nor Librosa installed"}))
             sys.exit(4)

    try:
        if not essentia_available:
            # Skip directly to librosa fallback
            raise ImportError("Essentia not available, using Librosa")
            
        # Load audio with Essentia
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
        # Fallback to Librosa if Essentia runtime failed OR wasn't available
        # We try Librosa in either case
        try:
            # Only log if we are falling back from a crash
            if essentia_available:
                # print(json.dumps({"warning": f"Essentia failed, falling back: {str(e)}"}))
                # Don't pollute stdout with warning if we want clean JSON, but debugging helps.
                # Actually, standard output should be strict JSON for the caller.
                # We can print to stderr.
                sys.stderr.write(f"Essentia failed: {e}\nFalling back to Librosa.\n")
                pass

            import librosa
            import numpy as np
            
            y, sr = librosa.load(audio_path)
            
            # BPM
            tempo, _ = librosa.beat.beat_track(y=y, sr=sr)
            bpm = float(tempo)
            
            # Features
            rms_val = float(np.mean(librosa.feature.rms(y=y)))
            sc = float(np.mean(librosa.feature.spectral_centroid(y=y, sr=sr)))
            zcr = float(np.mean(librosa.feature.zero_crossing_rate(y=y)))
            
            # Chromagram for key/mode detection
            chroma = librosa.feature.chroma_cqt(y=y, sr=sr)
            chroma_mean = np.mean(chroma, axis=1)
            
            # Estimate if song is in minor or major mode
            # Major/minor templates (Krumhansl-Schmuckler profiles simplified)
            major_profile = np.array([6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88])
            minor_profile = np.array([6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17])
            
            # Normalize profiles and chroma
            major_profile = major_profile / np.sum(major_profile)
            minor_profile = minor_profile / np.sum(minor_profile)
            chroma_norm = chroma_mean / (np.sum(chroma_mean) + 1e-8)
            
            # Correlation with profiles
            major_corr = np.corrcoef(chroma_norm, major_profile)[0, 1]
            minor_corr = np.corrcoef(chroma_norm, minor_profile)[0, 1]
            is_major = major_corr > minor_corr
            mode_confidence = abs(major_corr - minor_corr)
            
            # Spectral rolloff (darker = sadder)
            rolloff = float(np.mean(librosa.feature.spectral_rolloff(y=y, sr=sr)))
            darkness = max(0, 1.0 - rolloff / (sr / 2))  # Lower rolloff = darker
            
            # Approximations (Mirroring Essentia logic)
            danceability = min(1.0, max(0, 1 - abs(bpm - 120) / 60)) # Naive
            energy = min(1.0, max(0.0, (rms_val * 10 + sc / 5000) / 2))
            
            # IMPROVED Valence calculation
            # Components:
            # - Tempo: slower = sadder (but not too heavily weighted)
            # - Mode: minor = sadder
            # - Brightness: darker timbre = sadder
            # - Energy: lower energy can indicate sadness
            tempo_factor = min(1.0, bpm / 140)  # Normalize tempo (140 BPM = neutral)
            mode_factor = 0.65 if is_major else 0.20  # Major boost vs minor penalty
            brightness_factor = min(1.0, sc / 3000)  # Spectral centroid brightness
            
            valence = min(1.0, max(0.0,
                0.25 * tempo_factor +      # 25% tempo influence
                0.35 * mode_factor +       # 35% major/minor influence (most important!)
                0.25 * brightness_factor + # 25% timbre brightness
                0.15 * energy              # 15% energy
            )) 
            
            result = {
                "valence": round(valence, 4),
                "energy": round(energy, 4),
                "danceability": round(danceability, 4),
                "tempo": round(bpm, 2),
                "key": -1, # Librosa key detection is heavy/complex, skipping for pure-python/numpy speed
                "loudness": -10.0, 
                "instrumentalness": 0.0,
                "acousticness": 0.0,
                "speechiness": 0.0,
                "liveness": 0.0,
                "analysis_version": 1,
                "engine": "librosa"
            }
            print(json.dumps(result))
            sys.exit(0)
        except Exception as lib_e:
             error_msg = {"error": f"Analysis failed. Essentia: {str(e)}. Librosa: {str(lib_e)}"}
             print(json.dumps(error_msg))
             sys.exit(3)


if __name__ == "__main__":
    main()
