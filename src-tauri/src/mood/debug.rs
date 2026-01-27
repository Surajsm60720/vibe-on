/// Debug utilities for viewing mood analysis data during development
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize)]
pub struct DebugFeatureRow {
    pub track_path: String,
    pub valence: Option<f64>,
    pub energy: Option<f64>,
    pub danceability: Option<f64>,
    pub tempo: Option<f64>,
    pub key: Option<i32>,
    pub analysis_backend: Option<String>,
    pub analyzed_at: Option<String>,
    pub analysis_error: Option<String>,
}

/// Get all analyzed tracks with their features (for debugging)
pub fn get_all_features(conn: Arc<Mutex<Connection>>) -> Result<Vec<DebugFeatureRow>, String> {
    let conn = conn.lock().unwrap();
    
    let mut stmt = conn
        .prepare(
            "SELECT track_path, valence, energy, danceability, tempo, key, 
                    analysis_backend, analyzed_at, analysis_error
             FROM audio_features 
             ORDER BY analyzed_at DESC"
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(DebugFeatureRow {
                track_path: row.get(0)?,
                valence: row.get(1)?,
                energy: row.get(2)?,
                danceability: row.get(3)?,
                tempo: row.get(4)?,
                key: row.get(5)?,
                analysis_backend: row.get(6)?,
                analyzed_at: row.get(7)?,
                analysis_error: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut features = Vec::new();
    for row in rows {
        features.push(row.map_err(|e| e.to_string())?);
    }

    Ok(features)
}

/// Get statistics about analyzed tracks
#[derive(Debug, Serialize)]
pub struct AnalysisStats {
    pub total_tracks: u32,
    pub rust_analyzed: u32,
    pub python_analyzed: u32,
    pub errors: u32,
    pub avg_valence: Option<f64>,
    pub avg_energy: Option<f64>,
    pub avg_tempo: Option<f64>,
}

pub fn get_analysis_statistics(conn: Arc<Mutex<Connection>>) -> Result<AnalysisStats, String> {
    let conn = conn.lock().unwrap();

    let total_tracks: u32 = conn
        .query_row("SELECT COUNT(*) FROM audio_features", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let rust_analyzed: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM audio_features WHERE analysis_backend = 'rust'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let python_analyzed: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM audio_features WHERE analysis_backend = 'essentia'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let errors: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM audio_features WHERE analysis_error IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let avg_valence: Option<f64> = conn
        .query_row(
            "SELECT AVG(valence) FROM audio_features WHERE analysis_error IS NULL",
            [],
            |row| row.get(0),
        )
        .ok();

    let avg_energy: Option<f64> = conn
        .query_row(
            "SELECT AVG(energy) FROM audio_features WHERE analysis_error IS NULL",
            [],
            |row| row.get(0),
        )
        .ok();

    let avg_tempo: Option<f64> = conn
        .query_row(
            "SELECT AVG(tempo) FROM audio_features WHERE analysis_error IS NULL",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(AnalysisStats {
        total_tracks,
        rust_analyzed,
        python_analyzed,
        errors,
        avg_valence,
        avg_energy,
        avg_tempo,
    })
}
