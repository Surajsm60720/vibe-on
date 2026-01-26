use serde::{Deserialize, Serialize};

/// Audio features extracted from a track via Essentia
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AudioFeatures {
    /// Valence: 0.0 (sad/negative) to 1.0 (happy/positive)
    pub valence: f64,
    /// Energy: 0.0 (low) to 1.0 (high)
    pub energy: f64,
    /// Danceability: 0.0 (not danceable) to 1.0 (very danceable)
    pub danceability: f64,
    /// Tempo in BPM
    pub tempo: f64,
    /// Musical key: 0-11 (C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
    pub key: i32,
    /// Loudness in dB (typically -60 to 0)
    pub loudness: f64,
    /// Instrumentalness: 0.0 (vocal) to 1.0 (instrumental)
    pub instrumentalness: f64,
    /// Acousticness: 0.0 (electronic) to 1.0 (acoustic)
    pub acousticness: f64,
    /// Speechiness: 0.0 (music) to 1.0 (speech)
    pub speechiness: f64,
    /// Liveness: 0.0 (studio) to 1.0 (live recording)
    pub liveness: f64,
    /// Version of analysis algorithm (for cache invalidation)
    pub analysis_version: i32,
    /// When analysis was performed
    pub analyzed_at: Option<String>,
    /// Error message if analysis failed
    pub analysis_error: Option<String>,
}

/// Mood presets for radio/queue generation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MoodPreset {
    Happy,
    Sad,
    Energetic,
    Chill,
    Focus,
    Workout,
}

impl MoodPreset {
    /// Get the target feature ranges for this mood preset
    /// Returns (min, max) tuples for (valence, energy, danceability, tempo_min, tempo_max, instrumentalness_min)
    pub fn get_ranges(&self) -> MoodRanges {
        match self {
            MoodPreset::Happy => MoodRanges {
                valence: (0.6, 1.0),
                energy: (0.5, 1.0),
                danceability: (0.5, 1.0),
                tempo: (100.0, 140.0),
                instrumentalness: (0.0, 0.5),
            },
            MoodPreset::Sad => MoodRanges {
                valence: (0.0, 0.4),
                energy: (0.0, 0.5),
                danceability: (0.0, 0.5),
                tempo: (60.0, 100.0),
                instrumentalness: (0.0, 1.0),
            },
            MoodPreset::Energetic => MoodRanges {
                valence: (0.5, 1.0),
                energy: (0.7, 1.0),
                danceability: (0.6, 1.0),
                tempo: (120.0, 180.0),
                instrumentalness: (0.0, 0.5),
            },
            MoodPreset::Chill => MoodRanges {
                valence: (0.3, 0.7),
                energy: (0.0, 0.5),
                danceability: (0.2, 0.6),
                tempo: (70.0, 110.0),
                instrumentalness: (0.0, 1.0),
            },
            MoodPreset::Focus => MoodRanges {
                valence: (0.3, 0.7),
                energy: (0.3, 0.6),
                danceability: (0.2, 0.5),
                tempo: (90.0, 130.0),
                instrumentalness: (0.5, 1.0), // Prefer instrumental for focus
            },
            MoodPreset::Workout => MoodRanges {
                valence: (0.5, 1.0),
                energy: (0.8, 1.0),
                danceability: (0.7, 1.0),
                tempo: (130.0, 180.0),
                instrumentalness: (0.0, 0.5),
            },
        }
    }
}

/// Feature ranges for mood matching
#[derive(Clone, Debug)]
pub struct MoodRanges {
    pub valence: (f64, f64),
    pub energy: (f64, f64),
    pub danceability: (f64, f64),
    pub tempo: (f64, f64),
    pub instrumentalness: (f64, f64),
}

/// Analysis progress event for batch processing
#[derive(Clone, Debug, Serialize)]
pub struct AnalysisProgress {
    pub current: u32,
    pub total: u32,
    pub current_track: String,
    pub success_count: u32,
    pub error_count: u32,
}

/// Result of checking Essentia availability
#[derive(Clone, Debug, Serialize)]
pub struct EssentiaStatus {
    pub available: bool,
    pub python_version: Option<String>,
    pub essentia_version: Option<String>,
    pub error: Option<String>,
}

/// Current analysis version - increment when algorithm changes significantly
pub const ANALYSIS_VERSION: i32 = 1;
