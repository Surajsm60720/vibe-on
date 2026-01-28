mod audio;
mod cover_fetcher;
mod database;
mod discord_rpc;
mod lyrics_fetcher;
#[cfg(target_os = "windows")]
mod taskbar_controls;
mod torrent;
mod youtube_searcher;

// Mood-based playback feature (isolated module for clean removal)
mod mood;

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

use audio::state::PlayerStatus;
#[cfg(target_os = "windows")]
use audio::MediaControlService;
use audio::{AudioPlayer, MediaCmd, SearchFilter, TrackInfo, UnreleasedTrack};
use database::DatabaseManager;
use discord_rpc::DiscordRpc;

// Discord App ID
const DISCORD_APP_ID: &str = "1463457295974535241";

use std::sync::mpsc::Sender;

/// Cached lyrics for current track
#[derive(Clone, Default)]
pub struct CachedLyrics {
    pub track_path: String,
    pub synced_lyrics: Option<String>,
    pub plain_lyrics: Option<String>,
    pub instrumental: bool,
    pub is_fetching: bool,
    pub error: Option<String>,
}

/// Global player state managed by Tauri
pub struct AppState {
    player: Mutex<Option<AudioPlayer>>,
    db: Mutex<Option<DatabaseManager>>,
    discord: Arc<DiscordRpc>,
    current_cover_url: Arc<Mutex<Option<String>>>,
    media_cmd_tx: Mutex<Option<Sender<MediaCmd>>>,
    last_rpc_update: Mutex<String>, // De-duplication key
    lyrics_cache: Arc<Mutex<CachedLyrics>>,
    torrent_manager: Arc<Mutex<Option<torrent::TorrentManager>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            player: Mutex::new(None),
            db: Mutex::new(None),
            discord: Arc::new(DiscordRpc::new(DISCORD_APP_ID)),
            current_cover_url: Arc::new(Mutex::new(None)),
            media_cmd_tx: Mutex::new(None),
            last_rpc_update: Mutex::new(String::new()),
            lyrics_cache: Arc::new(Mutex::new(CachedLyrics::default())),
            torrent_manager: Arc::new(Mutex::new(None)),
        }
    }
}
/// Initialize the audio player
fn get_or_init_player(state: &AppState) -> Result<(), String> {
    let mut player_guard = state.player.lock().unwrap();
    if player_guard.is_none() {
        *player_guard = Some(AudioPlayer::new()?);
    }
    Ok(())
}

fn get_or_init_db(state: &AppState, app_handle: &AppHandle) -> Result<(), String> {
    let mut db_guard = state.db.lock().unwrap();
    if db_guard.is_none() {
        *db_guard = Some(DatabaseManager::new(app_handle).map_err(|e| e.to_string())?);
    }
    Ok(())
}

// ============================================================================
// Tauri Commands - Playback Control
// ============================================================================

#[tauri::command]
async fn play_file(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    println!("[Backend] play_file called with path: '{}'", path);
    get_or_init_player(&state)?;

    // CRITICAL: Start audio playback IMMEDIATELY for responsiveness
    {
        let player_guard = state.player.lock().unwrap();
        if let Some(ref player) = *player_guard {
            player.play_file(&path)?;
            println!("[Backend] Audio playback started immediately");
        } else {
            return Err("Player not initialized".to_string());
        }
    }

    // Now spawn background operations (Discord, lyrics, cover, media controls)
    // These don't block audio playback
    let path_clone = path.clone();
    let discord = state.discord.clone();
    let current_cover_url = state.current_cover_url.clone();
    let lyrics_cache = state.lyrics_cache.clone();
    let media_cmd_tx = state.media_cmd_tx.lock().unwrap().clone();
    let app_handle_thread = app_handle.clone();

    std::thread::spawn(move || {
        // Reset current cover
        if let Ok(mut url_guard) = current_cover_url.lock() {
            *url_guard = None;
        }

        // Reset lyrics cache and mark as fetching
        if let Ok(mut lyrics_guard) = lyrics_cache.lock() {
            println!("[Lyrics] Initializing cache for new track: {}", path_clone);
            *lyrics_guard = CachedLyrics {
                track_path: path_clone.clone(),
                is_fetching: true,
                ..Default::default()
            };
        }

        // Try to get metadata for Discord/lyrics/covers (single call, not duplicate)
        if let Ok((info, _)) = get_track_metadata_helper(&path_clone) {
            // Connect to Discord
            let _ = discord.connect();

            // Set initial Discord activity
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let _ = discord.set_activity(
                &info.title,
                &info.artist,
                Some(now),
                None,
                Some(info.album.clone()),
            );

            // Update Windows Media Controls
            if let Some(ref tx) = media_cmd_tx {
                let _ = tx.send(MediaCmd::SetMetadata {
                    title: info.title.clone(),
                    artist: info.artist.clone(),
                    album: info.album.clone(),
                });
                let _ = tx.send(MediaCmd::SetPlaying);
            }

            // Prefetch lyrics in separate thread
            let lyrics_cache_clone = lyrics_cache.clone();
            let artist = info.artist.clone();
            let track_title = info.title.clone();
            let duration = info.duration_secs as u32;
            let track_path = path_clone.clone();
            let app_h_lyrics = app_handle_thread.clone();

            std::thread::spawn(move || {
                println!(
                    "[Lyrics] Prefetching lyrics for: {} - {}",
                    artist, track_title
                );

                // Helper to emit progress
                let _emit_progress = |msg: &str| {
                    let _ = app_h_lyrics.emit("lyrics-loading-status", msg);
                };

                let app_h_1 = app_h_lyrics.clone();
                let cb1 = move |msg: &str| {
                    let _ = app_h_1.emit("lyrics-loading-status", msg);
                };
                let app_h_2 = app_h_lyrics.clone();
                let cb2 = move |msg: &str| {
                    let _ = app_h_2.emit("lyrics-loading-status", msg);
                };

                let result =
                    match lyrics_fetcher::fetch_lyrics(&artist, &track_title, duration, cb1) {
                        Ok(lyrics) => lyrics,
                        Err(_) => {
                            match lyrics_fetcher::fetch_lyrics_fallback(&artist, &track_title, cb2)
                            {
                                Ok(lyrics) => lyrics,
                                Err(e) => {
                                    if let Ok(mut guard) = lyrics_cache_clone.lock() {
                                        if guard.track_path == track_path {
                                            guard.is_fetching = false;
                                            guard.error = Some(e);
                                        }
                                    }
                                    return;
                                }
                            }
                        }
                    };

                if let Ok(mut guard) = lyrics_cache_clone.lock() {
                    if guard.track_path == track_path {
                        guard.synced_lyrics = result.synced_lyrics;
                        guard.plain_lyrics = result.plain_lyrics;
                        guard.instrumental = result.instrumental.unwrap_or(false);
                        guard.is_fetching = false;
                        guard.error = None;
                        println!(
                            "[Lyrics] Prefetch complete for: {} - {}",
                            artist, track_title
                        );
                    }
                }
            });

            // Cover fetch in separate thread
            let discord_clone = discord.clone();
            let url_mutex_clone = current_cover_url.clone();
            let artist = info.artist.clone();
            let album = info.album.clone();
            let title = info.title.clone();

            std::thread::spawn(move || {
                println!("[Cover] Searching for: {} - {}", artist, album);
                if let Some(url) = cover_fetcher::search_cover(&artist, &album) {
                    println!("[Cover] Found URL: {}", url);
                    if let Ok(mut guard) = url_mutex_clone.lock() {
                        *guard = Some(url.clone());
                    }

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    let _ = discord_clone.set_activity(
                        &title,
                        &artist,
                        Some(now),
                        Some(url),
                        Some(album),
                    );
                } else {
                    println!("[Cover] No cover found for: {} - {}", artist, album);
                }
            });
        } else {
            // Fallback: just set basic Discord activity
            let path_obj = std::path::Path::new(&path_clone);
            let filename = path_obj
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Track");
            let _ = discord.set_activity(filename, "Listening", None, None, None);
        }
    });

    Ok(())
}

#[tauri::command]
fn pause(state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        let status = player.get_status();
        let cover_url = state.current_cover_url.lock().unwrap().clone();

        if let Some(track) = status.track {
            let _ = state.discord.set_activity(
                &format!("(Paused) {}", track.title),
                &track.artist,
                None,
                cover_url,
                Some(track.album),
            );
        } else {
            let _ = state
                .discord
                .set_activity("Paused", "Vibe Music Player", None, None, None);
        }

        // Update Windows Media Controls
        if let Ok(tx_guard) = state.media_cmd_tx.lock() {
            if let Some(ref tx) = *tx_guard {
                let _ = tx.send(MediaCmd::SetPaused);
            }
        }

        player.pause()
    } else {
        Ok(())
    }
}

#[tauri::command]
fn resume(state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        let status = player.get_status();
        let cover_url = state.current_cover_url.lock().unwrap().clone();

        if let Some(track) = status.track {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            // When resuming, we need to calculate effective start time based on current position
            // status.position_secs is where we are.
            // effective_start = now - position
            let position = status.position_secs as i64;
            let start = now - position;

            let _ = state.discord.set_activity(
                &track.title,
                &track.artist,
                Some(start),
                cover_url,
                Some(track.album),
            );
        }

        // Update Windows Media Controls
        if let Ok(tx_guard) = state.media_cmd_tx.lock() {
            if let Some(ref tx) = *tx_guard {
                let _ = tx.send(MediaCmd::SetPlaying);
            }
        }

        player.resume()
    } else {
        Ok(())
    }
}

#[tauri::command]
fn stop(state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        let _ = state.discord.clear_activity();
        if let Ok(mut url_guard) = state.current_cover_url.lock() {
            *url_guard = None;
        }

        // Update Windows Media Controls
        if let Ok(tx_guard) = state.media_cmd_tx.lock() {
            if let Some(ref tx) = *tx_guard {
                let _ = tx.send(MediaCmd::SetStopped);
            }
        }

        player.stop()
    } else {
        Ok(())
    }
}

#[tauri::command]
fn set_volume(value: f32, state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        player.set_volume(value)
    } else {
        Ok(())
    }
}

#[tauri::command]
fn seek(value: f64, state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        player.seek(value)
    } else {
        Ok(())
    }
}

#[tauri::command]
fn set_eq(band: usize, gain: f32, state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        player.set_eq(band, gain)
    } else {
        Ok(())
    }
}

#[tauri::command]
fn get_player_state(state: State<AppState>) -> PlayerStatus {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        player.get_status()
    } else {
        PlayerStatus::default()
    }
}

// ============================================================================
// Tauri Commands - Library Management
// ============================================================================

#[tauri::command]
async fn init_library(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<Vec<TrackInfo>, String> {
    // 1. Init DB if needed
    get_or_init_db(&state, &app_handle)?;

    // 2. Scan folder (simple implementation for now calling the helper)
    // Note: In a real app this should probably be in a separate thread/task outside the async runtime block if blocking
    // But scan_music_folder_helper is sync.
    let path_obj = Path::new(&path);
    if !path_obj.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    println!("[Library] Scanning folder: {:?}", path_obj);
    let files = scan_music_folder_helper(path_obj);
    println!("[Library] Found {} files.", files.len());

    let mut tracks = Vec::new();
    let mut inserted_count = 0;

    // 3. Process metadata and insert into DB
    let db_guard = state.db.lock().unwrap();
    if let Some(ref db) = *db_guard {
        for file_path in files {
            // Re-use logic from get_track_metadata but we need it available here
            if let Ok((track, cover_data)) = get_track_metadata_helper(&file_path) {
                match db.insert_track(&track, cover_data.as_deref()) {
                    Ok(_) => inserted_count += 1,
                    Err(e) => eprintln!("[Library] Failed to insert track {}: {}", file_path, e),
                }
                tracks.push(track);
            }
        }
        println!(
            "[Library] Successfully processed {}/{} tracks.",
            inserted_count,
            tracks.len()
        );

        // Return all tracks currently in DB (or just the new ones? Use get_all_tracks for consistency)
        db.get_all_tracks().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_library_tracks(
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<Vec<TrackInfo>, String> {
    get_or_init_db(&state, &app_handle)?;
    let db_guard = state.db.lock().unwrap();
    if let Some(ref db) = *db_guard {
        db.get_all_tracks().map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn get_covers_dir(state: State<AppState>, app_handle: AppHandle) -> Result<String, String> {
    get_or_init_db(&state, &app_handle)?;
    let db_guard = state.db.lock().unwrap();
    if let Some(ref db) = *db_guard {
        Ok(db.get_covers_dir().to_string_lossy().to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

// Helper functions (extracted from previous commands)
fn scan_music_folder_helper(path: &Path) -> Vec<String> {
    let audio_extensions = ["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"];
    let mut files = Vec::new();

    fn scan_recursive(dir: &Path, extensions: &[&str], files: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // println!("Entering directory: {:?}", path);
                    scan_recursive(&path, extensions, files);
                } else if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str.to_lowercase().as_str()) {
                            if let Some(path_str) = path.to_str() {
                                files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        } else {
            eprintln!("Failed to read directory: {:?}", dir);
        }
    }

    scan_recursive(path, &audio_extensions, &mut files);
    files.sort();
    files
}

// Helper to find external cover image in the directory
fn find_external_cover(dir: &Path) -> Option<std::path::PathBuf> {
    let filenames = [
        "cover.jpg",
        "cover.png",
        "folder.jpg",
        "folder.png",
        "album.jpg",
        "album.png",
    ];
    for name in filenames.iter() {
        let p = dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn get_track_metadata_helper(path_str: &str) -> Result<(TrackInfo, Option<Vec<u8>>), String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let path = Path::new(path_str);
    let tagged_file_res = Probe::open(path)
        .map_err(|e| format!("Failed to probe file: {}", e))?
        .read();

    // Handle cases where reading tags fails but we still want the file
    // For now we error out if read fails, as before.
    let tagged_file = tagged_file_res.map_err(|e| format!("Failed to read metadata: {}", e))?;

    let properties = tagged_file.properties();
    let duration_secs = properties.duration().as_secs_f64();

    let (title, artist, album, disc_number, track_number) =
        if let Some(tag) = tagged_file.primary_tag() {
            (
                tag.title().map(|s| s.to_string()).unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string()
                }),
                tag.artist()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown Artist".to_string()),
                tag.album()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown Album".to_string()),
                tag.disk(),
                tag.track(),
            )
        } else {
            (
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                "Unknown Artist".to_string(),
                "Unknown Album".to_string(),
                None,
                None,
            )
        };

    // Extract picture
    let mut cover_data = tagged_file
        .primary_tag()
        .and_then(|tag| tag.pictures().first())
        .map(|pic| pic.data().to_vec());

    // Fallback to external cover if no embedded art
    if cover_data.is_none() {
        if let Some(parent) = path.parent() {
            if let Some(cover_path) = find_external_cover(parent) {
                if let Ok(data) = std::fs::read(cover_path) {
                    cover_data = Some(data);
                }
            }
        }
    }

    Ok((
        TrackInfo {
            path: path.to_string_lossy().to_string(),
            title,
            artist,
            album,
            duration_secs,
            cover_image: None, // Will be populated from DB later
            disc_number,
            track_number,
        },
        cover_data,
    ))
}

// Keep the old commands for now but scan_music_folder is now internal helper mostly
#[tauri::command]
fn scan_music_folder(path: String) -> Result<Vec<String>, String> {
    Ok(scan_music_folder_helper(Path::new(&path)))
}

#[tauri::command]
fn get_track_metadata(path: String) -> Result<TrackInfo, String> {
    get_track_metadata_helper(&path).map(|(info, _)| info)
}

// ============================================================================
// Lyrics Integration
// ============================================================================

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedLyricsResponse {
    pub synced_lyrics: Option<String>,
    pub plain_lyrics: Option<String>,
    pub instrumental: bool,
    pub is_fetching: bool,
    pub error: Option<String>,
    pub track_path: String,
}

/// Get cached lyrics for the currently playing track
/// Returns immediately with whatever is in the cache (may still be fetching)
#[tauri::command]
fn get_cached_lyrics(track_path: String, state: State<AppState>) -> CachedLyricsResponse {
    println!("[Lyrics] get_cached_lyrics called for: {}", track_path);

    if let Ok(guard) = state.lyrics_cache.lock() {
        println!("[Lyrics] Cache state - track_path: {}, is_fetching: {}, has_synced: {}, has_plain: {}, error: {:?}",
            guard.track_path,
            guard.is_fetching,
            guard.synced_lyrics.is_some(),
            guard.plain_lyrics.is_some(),
            guard.error
        );

        // Only return if the cached lyrics are for the requested track
        if guard.track_path == track_path {
            return CachedLyricsResponse {
                synced_lyrics: guard.synced_lyrics.clone(),
                plain_lyrics: guard.plain_lyrics.clone(),
                instrumental: guard.instrumental,
                is_fetching: guard.is_fetching,
                error: guard.error.clone(),
                track_path: guard.track_path.clone(),
            };
        } else {
            println!(
                "[Lyrics] Cache track mismatch: cached='{}', requested='{}'",
                guard.track_path, track_path
            );
        }
    }

    // No cached lyrics for this track
    CachedLyricsResponse {
        synced_lyrics: None,
        plain_lyrics: None,
        instrumental: false,
        is_fetching: false,
        error: Some("No lyrics cached for this track".to_string()),
        track_path,
    }
}

#[tauri::command]
async fn get_lyrics(
    audio_path: String,
    artist: String,
    track: String,
    duration: u32,
    app_handle: AppHandle,
) -> Result<lyrics_fetcher::LyricsResponse, String> {
    // Run in blocking thread as it uses reqwest::blocking
    let app_handle_thread = app_handle.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let app_h1 = app_handle_thread.clone();
        let cb1 = move |msg: &str| {
            let _ = app_h1.emit("lyrics-loading-status", msg);
        };

        // First check for local LRC file (instant!)
        // Check for local LRC file manually
        if let Some(local) = lyrics_fetcher::find_local_lrc(&audio_path) {
            cb1("Using local LRC file");
            return Ok(local);
        }

        let app_h2 = app_handle_thread.clone();
        let cb2 = move |msg: &str| {
            let _ = app_h2.emit("lyrics-loading-status", msg);
        };

        let app_h3 = app_handle_thread.clone();
        let cb3 = move |msg: &str| {
            let _ = app_h3.emit("lyrics-loading-status", msg);
        };

        // Then try API with duration
        match lyrics_fetcher::fetch_lyrics(&artist, &track, duration, cb2) {
            Ok(lyrics) => Ok(lyrics),
            Err(_) => {
                // Fallback: search without duration constraint
                lyrics_fetcher::fetch_lyrics_fallback(&artist, &track, cb3)
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn remove_folder(
    path: String,
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    get_or_init_db(&state, &app_handle)?;
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        db.remove_folder(&path).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn clear_all_data(state: State<AppState>, app_handle: AppHandle) -> Result<(), String> {
    get_or_init_db(&state, &app_handle)?;
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        db.clear_all_data().map_err(|e| e.to_string())?;

        // Also clear persisted torrent state (does not delete user downloads)
        if let Some(manager) = state.torrent_manager.lock().unwrap().as_ref() {
            let state_file = manager.download_dir.join(torrent::STATE_FILE);
            let _ = fs::remove_file(&state_file);
        }
        Ok(())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn apply_lrc_file(
    track_path: String,
    lrc_path: String,
    state: State<AppState>,
) -> Result<(), String> {
    let track_path = Path::new(&track_path);
    let lrc_source_path = Path::new(&lrc_path);

    if !track_path.exists() {
        return Err("Track file does not exist".to_string());
    }
    if !lrc_source_path.exists() {
        return Err("Selected LRC file does not exist".to_string());
    }

    // Determine destination path: same folder as track, same stem, .lrc extension
    let dest_path = track_path.with_extension("lrc");

    // Copy file
    std::fs::copy(lrc_source_path, &dest_path)
        .map_err(|e| format!("Failed to copy LRC file: {}", e))?;

    // Invalidate/Update cache if current track
    if let Ok(mut guard) = state.lyrics_cache.lock() {
        if guard.track_path == track_path.to_string_lossy() {
            // We can either clear it or try to reload immediately.
            // Clearing it is safer, frontend will re-fetch.
            guard.is_fetching = true;
            // Ideally we should reload the content here but reading file again is easy enough for next fetch
        }
    }

    Ok(())
}

// ============================================================================
// YouTube Music Integration
// ============================================================================

#[tauri::command]
async fn open_yt_music(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    use tauri::Manager;
    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;

    // Check if webview already exists
    if let Some(webview) = app.get_webview_window("ytmusic") {
        let _ = webview.set_focus();
        return Ok(());
    }

    let _main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Create child webview attached to main window
    // Initial size/position will be set by move_yt_window immediately after

    // We update to WebviewWindowBuilder for Tauri v2 compatibility
    #[allow(unused_mut)] // Mutable only on Windows
    let mut builder = WebviewWindowBuilder::new(
        &app,
        "ytmusic",
        WebviewUrl::External("https://music.youtube.com".parse().unwrap())
    )
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
    .devtools(true)
    .initialization_script(include_str!("yt_inject.js"))
    .inner_size(width, height)
    .visible(false); // Start hidden

    #[cfg(target_os = "windows")]
    {
        builder = builder.parent(&_main_window).map_err(|e| e.to_string())?;
    }

    builder.build().map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Torrent Integration
// ============================================================================

#[tauri::command]
async fn init_torrent_backend(
    download_dir: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dir = std::path::PathBuf::from(download_dir);
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }

    // Use a block to Scope the lock
    // Check if initialized
    let needs_init = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.is_none()
    };

    if needs_init {
        let manager = torrent::TorrentManager::new(dir)
            .await
            .map_err(|e| e.to_string())?;

        let mut guard = state.torrent_manager.lock().unwrap();
        if guard.is_none() {
            *guard = Some(manager);
        }
    }
    Ok(())
}

#[tauri::command]
async fn add_magnet_link(
    magnet: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };

    if let Some(manager) = manager {
        let download_path =
            path.unwrap_or_else(|| manager.download_dir.to_string_lossy().to_string());
        manager
            .add_torrent(Some(magnet), None, download_path, None)
            .await
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

#[tauri::command]
async fn get_torrents(state: State<'_, AppState>) -> Result<Vec<torrent::TorrentStatus>, String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };

    if let Some(manager) = manager {
        Ok(manager.get_all_status())
    } else {
        Ok(Vec::new())
    }
}

// ============================================================================

/// Response from inspect commands
#[derive(serde::Serialize)]
pub struct InspectResult {
    pub name: String,
    pub files: Vec<torrent::TorrentFile>,
}

#[tauri::command]
async fn inspect_magnet(
    magnet: String,
    state: State<'_, AppState>,
) -> Result<InspectResult, String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };
    if let Some(manager) = manager {
        let (name, files) = manager.inspect_magnet(&magnet).await?;
        Ok(InspectResult { name, files })
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

#[tauri::command]
async fn inspect_torrent_file(
    data: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<InspectResult, String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };
    if let Some(manager) = manager {
        let (name, files) = manager.inspect_torrent_file(data).await?;
        Ok(InspectResult { name, files })
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

#[tauri::command]
async fn add_torrent_with_options(
    magnet: Option<String>,
    file_bytes: Option<Vec<u8>>,
    path: String,
    selected_files: Option<Vec<usize>>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };
    if let Some(manager) = manager {
        manager
            .add_torrent(magnet, file_bytes, path, selected_files)
            .await
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

#[tauri::command]
async fn delete_torrent(
    id: usize,
    delete_files: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };
    if let Some(manager) = manager {
        manager.delete(id, delete_files).await
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

#[tauri::command]
async fn pause_torrent(id: usize, state: State<'_, AppState>) -> Result<(), String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };
    if let Some(manager) = manager {
        manager.pause(id).await
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

#[tauri::command]
async fn resume_torrent(id: usize, state: State<'_, AppState>) -> Result<(), String> {
    let manager = {
        let guard = state.torrent_manager.lock().unwrap();
        guard.clone()
    };
    if let Some(manager) = manager {
        manager.resume(id).await
    } else {
        Err("Torrent backend not initialized".to_string())
    }
}

// ============================================================================
// Unreleased Library Integration
// ============================================================================

#[tauri::command]
async fn search_youtube(filter: SearchFilter) -> Result<Vec<UnreleasedTrack>, String> {
    // Run in blocking thread as reqwest::blocking is used
    tauri::async_runtime::spawn_blocking(move || youtube_searcher::search_youtube(filter))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn save_unreleased_track(
    track: UnreleasedTrack,
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    get_or_init_db(&state, &app_handle)?;
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        db.insert_unreleased_track(&track)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn remove_unreleased_track(
    video_id: String,
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    get_or_init_db(&state, &app_handle)?;
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        db.delete_unreleased_track(&video_id)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_unreleased_library(
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<Vec<UnreleasedTrack>, String> {
    get_or_init_db(&state, &app_handle)?;
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        db.get_unreleased_tracks().map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct YtStatus {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover_url: String,
    pub duration: f64,
    pub progress: f64,
    pub is_playing: bool,
}

#[tauri::command]
fn update_yt_status(
    status: YtStatus,
    state: State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // We construct a key to check effective change.
    // Including is_playing is crucial.
    // We DON'T include progress in the key because progress changes every second, but we don't want to re-set activity every second if the "Activity" itself (playing Song X) hasn't changed.
    // However, for "Time Remaining" to be accurate if the user seeks, we might want to update.
    // But re-setting activity continuously might be bad.
    // Let's rely on Title+Artist+State change.

    let key = format!("{}|{}|{}", status.title, status.artist, status.is_playing);

    let mut should_update = false;
    if let Ok(mut last) = state.last_rpc_update.lock() {
        if *last != key {
            *last = key;
            should_update = true;
        }
    }

    if should_update {
        let _ = state.discord.set_activity(
            &status.title,
            &status.artist,
            if status.is_playing {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let start = now - status.progress as i64;
                Some(start)
            } else {
                None
            },
            if status.cover_url.is_empty() {
                None
            } else {
                Some(status.cover_url.clone())
            },
            Some(status.album.clone()),
        );
    }

    // 2. Emit event to Frontend (so PlayerBar updates)
    app.emit("player:update", &status)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn yt_control(action: String, value: Option<f64>, app: AppHandle) -> Result<(), String> {
    // use tauri::Manager;
    // Need to use get_webview for child webviews
    if let Some(webview) = app.get_webview_window("ytmusic") {
        let js = format!("window.ytControl('{}', {})", action, value.unwrap_or(0.0));
        webview.eval(&js).map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn yt_navigate(url: String, app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;

    // Create webview if it doesn't exist
    if app.get_webview_window("ytmusic").is_none() {
        let builder = WebviewWindowBuilder::new(
            &app,
            "ytmusic",
            WebviewUrl::External(url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
        )
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .devtools(true)
        .initialization_script(include_str!("yt_inject.js"))
        .inner_size(800.0, 600.0)
        .visible(true);

        builder.build().map_err(|e| e.to_string())?;
    } else {
        // Webview exists, just navigate and show it
        if let Some(webview) = app.get_webview_window("ytmusic") {
            // Navigate by evaluating JavaScript
            let js = format!("window.location.href = '{}';", url);
            webview.eval(&js).map_err(|e: tauri::Error| e.to_string())?;
            webview.show().map_err(|e: tauri::Error| e.to_string())?;
            webview
                .set_focus()
                .map_err(|e: tauri::Error| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn set_yt_visibility(show: bool, app: AppHandle) -> Result<(), String> {
    // use tauri::Manager;
    if let Some(webview) = app.get_webview_window("ytmusic") {
        if show {
            webview.show().map_err(|e: tauri::Error| e.to_string())?;
            webview
                .set_focus()
                .map_err(|e: tauri::Error| e.to_string())?;
        } else {
            webview.hide().map_err(|e: tauri::Error| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn move_yt_window(x: f64, y: f64, width: f64, height: f64, app: AppHandle) -> Result<(), String> {
    // use tauri::Manager;
    if let Some(webview) = app.get_webview_window("ytmusic") {
        // For child webviews, bounds are relative to the parent window's content area (I think?).
        // If x, y are screen coordinates (which they were before), we need to convert them?
        // Or if the frontend sends us screen coordinates, we need to map them to window-relative coordinates.
        // Wait, the previous implementation received Physical coordinates which were presumably screen coordinates.
        // But for a child webview, we want coordinates relative to the Main Window.

        // Frontend logic in YouTubeMusic.tsx:
        // const x = winPos.x + Math.round(rect.x * factor);
        // const y = winPos.y + Math.round(rect.y * factor);
        // This calculates SCREEN coordinates.

        // If we use Child Webview, we want RELATIVE coordinates (rect.x, rect.y).
        // So we need to update the Frontend too.
        // However, if we just use set_bounds with what we have, it might be offset by the window position.
        // Let's assume x, y are correct logical coordinates relative to the window for now,
        // OR we'll fix the frontend to send relative coordinates.
        // Since we are changing the implementation significantly, we should update the frontend.

        // Actually, let's keep the signature but expect RELATIVE coordinates (Logical) from now on?
        // Or Physical relative.

        // Let's assume input is Logical or Physical PIXELS.
        // Webview::set_bounds takes a Rect.

        // Use LogicalPosition and LogicalSize as that's likely what we get from JS, or just map directly

        webview
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: x as i32,
                y: y as i32,
            }))
            .map_err(|e: tauri::Error| e.to_string())?;

        webview
            .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: width as u32,
                height: height as u32,
            }))
            .map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// App Entry Point
// ============================================================================

#[tauri::command]
async fn search_torrents(
    query: String,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<Vec<torrent::search::SearchResult>, String> {
    torrent::search::search_nyaa(query, sort_by, sort_order).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Threading::*;
        SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS).unwrap_or_default();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            play_file,
            pause,
            resume,
            stop,
            set_volume,
            seek,
            get_player_state,
            scan_music_folder,
            get_track_metadata,
            init_library,
            get_library_tracks,
            get_covers_dir,
            open_yt_music,
            update_yt_status,
            yt_control,
            yt_navigate,
            set_yt_visibility,
            move_yt_window,
            get_lyrics,
            get_cached_lyrics,
            search_youtube,
            save_unreleased_track,
            remove_unreleased_track,
            get_unreleased_library,
            remove_folder,
            clear_all_data,
            apply_lrc_file,
            init_torrent_backend,
            add_magnet_link,
            inspect_magnet,
            inspect_torrent_file,
            add_torrent_with_options,
            delete_torrent,
            // Torrent Control
            get_torrents,
            pause_torrent,
            resume_torrent,
            search_torrents,
            // Mood feature commands
            mood::check_essentia_available,
            mood::analyze_track,
            mood::get_track_audio_features,
            mood::analyze_library,
            mood::cancel_analysis,
            mood::get_mood_radio_queue,
            mood::get_similar_tracks,
            mood::get_analysis_stats,
            mood::clear_analysis_data,
            #[cfg(debug_assertions)]
            mood::debug_get_all_features,
            #[cfg(debug_assertions)]
            mood::debug_get_statistics,
            search_torrents,
            // Equalizer
            set_eq,
        ])
        .setup(|_app| {
            // Initialize Windows Media Controls with the main window handle
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;

                if let Some(window) = _app.get_webview_window("main") {
                    // Initialize Taskbar Buttons (Thumbnail Toolbar)
                    taskbar_controls::init(window.clone());

                    // Get HWND from the window
                    let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);

                    if hwnd != 0 {
                        // Pass 0 (None) for HWND to MediaControlService as per previous fix
                        // Use app.handle().clone()
                        let tx = MediaControlService::start(_app.handle().clone(), 0);
                        let state = _app.state::<AppState>();

                        match state.media_cmd_tx.lock() {
                            Ok(mut tx_guard) => {
                                *tx_guard = Some(tx);
                                println!("[MediaControls] Service started successfully");
                            }
                            Err(e) => {
                                eprintln!("[MediaControls] Failed to lock mutex: {}", e);
                            }
                        };
                    }
                }
            }

            // Initialize Mood features
            mood::setup_mood_state(_app.handle());

            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            println!("Second instance launched");
        }))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
