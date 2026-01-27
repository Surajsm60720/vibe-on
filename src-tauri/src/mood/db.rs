use rusqlite::{params, Connection, OptionalExtension, Result};
use std::sync::{Arc, Mutex};

use super::schema::init_audio_features_table;
use super::types::{AudioFeatures, MoodPreset, ANALYSIS_VERSION};

/// Database operations for audio features
/// Completely isolated from main database module for clean removal
pub struct MoodDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl MoodDatabase {
    /// Create a new MoodDatabase with its own connection to the database file
    pub fn new_from_path(db_path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        init_audio_features_table(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a new MoodDatabase wrapper around an existing connection
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        {
            let c = conn.lock().unwrap();
            init_audio_features_table(&c)?;
        }
        Ok(Self { conn })
    }

    /// Get access to the underlying database connection for debug utilities
    #[cfg(debug_assertions)]
    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Insert or update audio features for a track
    pub fn upsert_features(&self, track_path: &str, features: &AudioFeatures) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audio_features (track_path, valence, energy, danceability, tempo, key, loudness, 
             instrumentalness, acousticness, speechiness, liveness, analysis_version, analysis_error, analysis_backend)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(track_path) DO UPDATE SET
             valence = excluded.valence,
             energy = excluded.energy,
             danceability = excluded.danceability,
             tempo = excluded.tempo,
             key = excluded.key,
             loudness = excluded.loudness,
             instrumentalness = excluded.instrumentalness,
             acousticness = excluded.acousticness,
             speechiness = excluded.speechiness,
             liveness = excluded.liveness,
             analysis_version = excluded.analysis_version,
             analyzed_at = CURRENT_TIMESTAMP,
             analysis_error = excluded.analysis_error,
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
                &features.analysis_backend,
            ],
        )?;
        Ok(())
    }

    /// Get audio features for a track
    pub fn get_features(&self, track_path: &str) -> Result<Option<AudioFeatures>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT valence, energy, danceability, tempo, key, loudness, instrumentalness,
             acousticness, speechiness, liveness, analysis_version, analyzed_at, analysis_error, analysis_backend
             FROM audio_features WHERE track_path = ?1",
            params![track_path],
            |row| {
                Ok(AudioFeatures {
                    valence: row.get(0)?,
                    energy: row.get(1)?,
                    danceability: row.get(2)?,
                    tempo: row.get(3)?,
                    key: row.get(4)?,
                    loudness: row.get(5)?,
                    instrumentalness: row.get(6)?,
                    acousticness: row.get(7)?,
                    speechiness: row.get(8)?,
                    liveness: row.get(9)?,
                    analysis_version: row.get(10)?,
                    analyzed_at: row.get(11)?,
                    analysis_error: row.get(12)?,
                    analysis_backend: row.get(13)?,
                })
            },
        )
        .optional()
    }

    /// Mark a track as having an analysis error
    pub fn mark_error(&self, track_path: &str, error: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audio_features (track_path, analysis_error, analysis_version)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(track_path) DO UPDATE SET
             analysis_error = excluded.analysis_error,
             analyzed_at = CURRENT_TIMESTAMP",
            params![track_path, error, ANALYSIS_VERSION],
        )?;
        Ok(())
    }

    /// Get list of track paths that need analysis
    /// Returns tracks without features or with outdated version
    pub fn get_tracks_needing_analysis(&self, all_tracks: &[String], limit: usize) -> Vec<String> {
        let conn = self.conn.lock().unwrap();
        let mut result = Vec::new();

        for path in all_tracks {
            if result.len() >= limit {
                break;
            }

            // Check if track has current version features without error
            let needs_analysis: bool = conn
                .query_row(
                    "SELECT 1 FROM audio_features 
                     WHERE track_path = ?1 
                     AND analysis_version = ?2 
                     AND analysis_error IS NULL",
                    params![path, ANALYSIS_VERSION],
                    |_| Ok(false),
                )
                .unwrap_or(true);

            if needs_analysis {
                result.push(path.clone());
            }
        }

        result
    }

    /// Find tracks matching a mood preset
    pub fn get_tracks_by_mood(&self, preset: &MoodPreset, limit: usize) -> Result<Vec<String>> {
        let ranges = preset.get_ranges();
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT track_path FROM audio_features
             WHERE valence BETWEEN ?1 AND ?2
             AND energy BETWEEN ?3 AND ?4
             AND danceability BETWEEN ?5 AND ?6
             AND tempo BETWEEN ?7 AND ?8
             AND instrumentalness BETWEEN ?9 AND ?10
             AND analysis_error IS NULL
             ORDER BY RANDOM()
             LIMIT ?11",
        )?;

        let paths = stmt
            .query_map(
                params![
                    ranges.valence.0,
                    ranges.valence.1,
                    ranges.energy.0,
                    ranges.energy.1,
                    ranges.danceability.0,
                    ranges.danceability.1,
                    ranges.tempo.0,
                    ranges.tempo.1,
                    ranges.instrumentalness.0,
                    ranges.instrumentalness.1,
                    limit as i32,
                ],
                |row| row.get(0),
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(paths)
    }

    /// Find similar tracks based on feature distance
    pub fn find_similar_tracks(
        &self,
        source_features: &AudioFeatures,
        exclude_path: &str,
        limit: usize,
    ) -> Result<Vec<(String, f64)>> {
        let conn = self.conn.lock().unwrap();

        // Weighted Euclidean distance - prioritize valence and energy
        let mut stmt = conn.prepare(
            "SELECT track_path,
             (2.0 * (valence - ?1) * (valence - ?1) + 
              2.0 * (energy - ?2) * (energy - ?2) + 
              (danceability - ?3) * (danceability - ?3) + 
              0.0001 * (tempo - ?4) * (tempo - ?4) +
              (instrumentalness - ?5) * (instrumentalness - ?5)) as distance
             FROM audio_features
             WHERE track_path != ?6 AND analysis_error IS NULL
             ORDER BY distance ASC
             LIMIT ?7",
        )?;

        let results = stmt
            .query_map(
                params![
                    source_features.valence,
                    source_features.energy,
                    source_features.danceability,
                    source_features.tempo,
                    source_features.instrumentalness,
                    exclude_path,
                    limit as i32,
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Get count of analyzed tracks
    pub fn get_analysis_stats(&self) -> Result<(u32, u32)> {
        let conn = self.conn.lock().unwrap();
        let success: u32 = conn.query_row(
            "SELECT COUNT(*) FROM audio_features WHERE analysis_error IS NULL",
            [],
            |row| row.get(0),
        )?;
        let errors: u32 = conn.query_row(
            "SELECT COUNT(*) FROM audio_features WHERE analysis_error IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok((success, errors))
    }

    /// Clear all audio features (for testing or reset)
    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM audio_features", [])?;
        Ok(())
    }
}
