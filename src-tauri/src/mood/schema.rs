/// SQL schema for audio features table
/// Kept separate from main schema for clean module removal
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
    analysis_error TEXT
);

CREATE INDEX IF NOT EXISTS idx_audio_features_path ON audio_features(track_path);
"#;

use rusqlite::{Connection, Result};

/// Initialize the audio features table
/// Call this from main DB init when mood feature is enabled
pub fn init_audio_features_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(AUDIO_FEATURES_SCHEMA)
}
