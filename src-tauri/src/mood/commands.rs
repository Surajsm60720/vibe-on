use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State};

use super::analyzer::AudioAnalyzer;
use super::db::MoodDatabase;
use super::types::{AnalysisProgress, AudioFeatures, EssentiaStatus, MoodPreset};

/// Shared state for mood feature
pub struct MoodState {
    pub db: Option<MoodDatabase>,
    pub analyzer: Option<AudioAnalyzer>,
    pub analysis_cancel: Arc<Mutex<bool>>,
}

impl Default for MoodState {
    fn default() -> Self {
        // Get the directory containing the executable to find the sidecar
        let exe_path = std::env::current_exe().ok();
        let resources_dir = exe_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());

        let analyzer = resources_dir.as_ref().map(|dir| AudioAnalyzer::new(dir));

        // Try to open connection to the app's database
        // On macOS: ~/Library/Application Support/moe.memesta.vibe-on/library.db
        let db = dirs::data_dir()
            .map(|d| d.join("moe.memesta.vibe-on").join("library.db"))
            .filter(|p| p.exists())
            .and_then(|db_path| {
                println!("[Mood] Opening database at: {:?}", db_path);
                MoodDatabase::new_from_path(&db_path).ok()
            });

        if db.is_none() {
            println!("[Mood] Warning: Could not open database, mood features will be limited");
        }

        Self {
            db,
            analyzer,
            analysis_cancel: Arc::new(Mutex::new(false)),
        }
    }
}

/// Initialize mood feature with database connection
pub fn init_mood_state(conn: Arc<Mutex<Connection>>, resources_dir: &Path) -> MoodState {
    let db = MoodDatabase::new(conn).ok();
    let analyzer = Some(AudioAnalyzer::new(resources_dir));

    MoodState {
        db,
        analyzer,
        analysis_cancel: Arc::new(Mutex::new(false)),
    }
}

// ============================================================================
// Tauri Commands - All mood-related commands isolated here
// ============================================================================

/// Check if Essentia/Python are available
#[tauri::command]
pub fn check_essentia_available(state: State<MoodState>) -> EssentiaStatus {
    match &state.analyzer {
        Some(analyzer) => analyzer.check_availability(),
        None => EssentiaStatus {
            available: false,
            python_version: None,
            essentia_version: None,
            error: Some("Mood feature not initialized".to_string()),
        },
    }
}

/// Analyze a single track
#[tauri::command]
pub async fn analyze_track(
    path: String,
    state: State<'_, MoodState>,
) -> Result<AudioFeatures, String> {
    let analyzer = state.analyzer.as_ref().ok_or("Analyzer not initialized")?;

    let db = state.db.as_ref().ok_or("Database not initialized")?;

    // Check cache first
    if let Ok(Some(features)) = db.get_features(&path) {
        if features.analysis_error.is_none() {
            return Ok(features);
        }
    }

    // Run analysis
    match analyzer.analyze_track(&path) {
        Ok(features) => {
            // Cache result
            let _ = db.upsert_features(&path, &features);
            Ok(features)
        }
        Err(e) => {
            // Mark error in DB to avoid repeated failures
            let _ = db.mark_error(&path, &e);
            Err(e)
        }
    }
}

/// Get cached audio features for a track
#[tauri::command]
pub fn get_track_audio_features(
    path: String,
    state: State<MoodState>,
) -> Result<Option<AudioFeatures>, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    db.get_features(&path).map_err(|e| e.to_string())
}

/// Start batch library analysis with progress events
#[tauri::command]
pub async fn analyze_library(
    track_paths: Vec<String>,
    app_handle: AppHandle,
    state: State<'_, MoodState>,
) -> Result<(u32, u32), String> {
    let analyzer = state.analyzer.as_ref().ok_or("Analyzer not initialized")?;

    let db = state.db.as_ref().ok_or("Database not initialized")?;

    // Reset cancel flag
    *state.analysis_cancel.lock().unwrap() = false;

    // Get tracks that need analysis
    let pending = db.get_tracks_needing_analysis(&track_paths, track_paths.len());
    let total = pending.len() as u32;

    if total == 0 {
        return Ok((0, 0));
    }

    let mut success_count = 0u32;
    let mut error_count = 0u32;
    let cancel_flag = state.analysis_cancel.clone();

    for (i, path) in pending.iter().enumerate() {
        // Check for cancellation
        if *cancel_flag.lock().unwrap() {
            break;
        }

        // Emit progress event
        let progress = AnalysisProgress {
            current: i as u32 + 1,
            total,
            current_track: path.clone(),
            success_count,
            error_count,
        };
        let _ = app_handle.emit("mood:analysis_progress", &progress);

        // Analyze track
        match analyzer.analyze_track(path) {
            Ok(features) => {
                let _ = db.upsert_features(path, &features);
                success_count += 1;
            }
            Err(e) => {
                let _ = db.mark_error(path, &e);
                error_count += 1;
            }
        }
    }

    // Emit completion event
    let _ = app_handle.emit("mood:analysis_complete", (success_count, error_count));

    Ok((success_count, error_count))
}

/// Cancel ongoing library analysis
#[tauri::command]
pub fn cancel_analysis(state: State<MoodState>) {
    *state.analysis_cancel.lock().unwrap() = true;
}

/// Get tracks matching a mood preset
#[tauri::command]
pub fn get_mood_radio_queue(
    preset: String,
    limit: u32,
    state: State<MoodState>,
) -> Result<Vec<String>, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;

    let mood_preset = match preset.to_lowercase().as_str() {
        "happy" => MoodPreset::Happy,
        "sad" => MoodPreset::Sad,
        "energetic" => MoodPreset::Energetic,
        "chill" => MoodPreset::Chill,
        "focus" => MoodPreset::Focus,
        "workout" => MoodPreset::Workout,
        _ => return Err(format!("Unknown mood preset: {}", preset)),
    };

    db.get_tracks_by_mood(&mood_preset, limit as usize)
        .map_err(|e| e.to_string())
}

/// Get similar tracks based on current track features
/// Auto-analyzes source track if not yet analyzed
#[tauri::command]
pub async fn get_similar_tracks(
    source_path: String,
    limit: u32,
    state: State<'_, MoodState>,
) -> Result<Vec<String>, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    let analyzer = state.analyzer.as_ref().ok_or("Analyzer not initialized")?;

    // Try to get cached features
    let source_features = match db
        .get_features(&source_path)
        .map_err(|e| e.to_string())?
    {
        Some(features) if features.analysis_error.is_none() => features,
        _ => {
            // Auto-analyze the source track if not cached or has error
            println!("[Mood] Auto-analyzing source track: {}", source_path);
            match analyzer.analyze_track(&source_path) {
                Ok(features) => {
                    let _ = db.upsert_features(&source_path, &features);
                    features
                }
                Err(e) => {
                    let _ = db.mark_error(&source_path, &e);
                    return Err(format!("Failed to analyze source track: {}", e));
                }
            }
        }
    };

    let similar = db
        .find_similar_tracks(&source_features, &source_path, limit as usize)
        .map_err(|e| e.to_string())?;

    Ok(similar.into_iter().map(|(path, _)| path).collect())
}

/// Get analysis statistics
#[tauri::command]
pub fn get_analysis_stats(state: State<MoodState>) -> Result<(u32, u32), String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    db.get_analysis_stats().map_err(|e| e.to_string())
}

/// Clear all analysis data
#[tauri::command]
pub fn clear_analysis_data(state: State<MoodState>) -> Result<(), String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    db.clear_all().map_err(|e| e.to_string())
}
