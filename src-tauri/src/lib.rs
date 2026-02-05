mod audio;
mod cover_fetcher;
mod database;
mod discord_rpc;
mod lyrics_fetcher;
mod p2p;
mod server;
#[cfg(target_os = "windows")]
mod taskbar_controls;
mod torrent;

// Mood-based playback feature (isolated module for clean removal)
mod mood;

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

use audio::state::PlayerStatus;
#[cfg(target_os = "windows")]
use audio::MediaControlService;
use audio::{AudioPlayer, MediaCmd, TrackInfo};
use database::DatabaseManager;
use discord_rpc::DiscordRpc;
use p2p::P2PManager;
use tokio::sync::RwLock as TokioRwLock;

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
    lyrics_cache: Arc<Mutex<CachedLyrics>>,
    torrent_manager: Arc<Mutex<Option<torrent::TorrentManager>>>,
    p2p_manager: Arc<TokioRwLock<Option<P2PManager>>>,
    server_running: Arc<Mutex<bool>>,
    server_shutdown_tx: Arc<Mutex<Option<tokio::sync::broadcast::Sender<()>>>>,
    // --- Queue Management ---
    pub queue: Arc<Mutex<Vec<TrackInfo>>>,
    pub current_queue_index: Arc<Mutex<usize>>,
    pub shuffle: Arc<Mutex<bool>>,
    pub repeat_mode: Arc<Mutex<String>>, // "off", "one", "all"
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            player: Mutex::new(None),
            db: Mutex::new(None),
            discord: Arc::new(DiscordRpc::new(DISCORD_APP_ID)),
            current_cover_url: Arc::new(Mutex::new(None)),
            media_cmd_tx: Mutex::new(None),
            lyrics_cache: Arc::new(Mutex::new(CachedLyrics::default())),
            torrent_manager: Arc::new(Mutex::new(None)),
            p2p_manager: Arc::new(TokioRwLock::new(None)),
            server_running: Arc::new(Mutex::new(false)),
            server_shutdown_tx: Arc::new(Mutex::new(None)),
            queue: Arc::new(Mutex::new(Vec::new())),
            current_queue_index: Arc::new(Mutex::new(0)),
            shuffle: Arc::new(Mutex::new(false)),
            repeat_mode: Arc::new(Mutex::new("off".to_string())),
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
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // 1. Init DB if needed
    get_or_init_db(&state, &app_handle)?;

    let path_obj = Path::new(&path);
    if !path_obj.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    println!("[Library] Scanning folder: {:?}", path_obj);
    let files = scan_music_folder_helper(path_obj);
    println!(
        "[Library] Found {} files. Processing in parallel...",
        files.len()
    );

    let processed = AtomicUsize::new(0);
    let total = files.len();

    // 2. Process metadata IN PARALLEL (skip cover extraction for speed)
    let tracks: Vec<TrackInfo> = files
        .par_iter()
        .filter_map(|file_path| {
            let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
            if count % 100 == 0 || count == total {
                println!("[Library] Processed {}/{} files...", count, total);
            }

            // Extract metadata WITHOUT cover art (much faster)
            match get_track_metadata_helper_fast(file_path) {
                Ok(track) => Some(track),
                Err(_) => None, // Skip files that fail
            }
        })
        .collect();

    println!(
        "[Library] Metadata extraction complete. Inserting {} tracks into database...",
        tracks.len()
    );

    // 3. Batch insert into database
    let db_guard = state.db.lock().unwrap();
    if let Some(ref db) = *db_guard {
        let mut inserted_count = 0;
        for track in &tracks {
            // Insert without cover data initially (covers loaded lazily on demand)
            match db.insert_track(&track, None) {
                Ok(_) => inserted_count += 1,
                Err(e) => eprintln!("[Library] Failed to insert track {}: {}", track.path, e),
            }
        }
        println!(
            "[Library] Successfully inserted {}/{} tracks.",
            inserted_count,
            tracks.len()
        );

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
        let covers_dir_path = db.get_covers_dir().to_string_lossy().to_string();
        println!("Rust Backend: coversDir resolved to: {}", covers_dir_path);
        Ok(covers_dir_path)
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

// Fast metadata extraction WITHOUT cover art (for bulk import)
fn get_track_metadata_helper_fast(path_str: &str) -> Result<TrackInfo, String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let path = Path::new(path_str);
    let tagged_file_res = Probe::open(path).and_then(|probe| probe.read());

    let tagged_file = tagged_file_res.map_err(|e| format!("{}", e))?;

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

    Ok(TrackInfo {
        path: path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_secs,
        cover_image: None,
        disc_number,
        track_number,
    })
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
    println!("[clear_all_data] Starting complete data clear...");

    // Stop any playing audio first
    if let Ok(player_guard) = state.player.lock() {
        if let Some(player) = player_guard.as_ref() {
            let _ = player.stop();
            println!("[clear_all_data] Stopped player");
        }
    }

    // Clear database and covers
    get_or_init_db(&state, &app_handle)?;
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        db.clear_all_data().map_err(|e| e.to_string())?;

        // Also clear persisted torrent state (does not delete user downloads)
        if let Some(manager) = state.torrent_manager.lock().unwrap().as_ref() {
            let state_file = manager.download_dir.join(torrent::STATE_FILE);
            let _ = fs::remove_file(&state_file);
        }
        println!("[clear_all_data] Database cleared");
    } else {
        return Err("Database not initialized".to_string());
    }

    // Clear lyrics cache
    if let Ok(mut lyrics_guard) = state.lyrics_cache.lock() {
        *lyrics_guard = CachedLyrics::default();
        println!("[clear_all_data] Lyrics cache cleared");
    }

    // Clear app data directory (settings, cache, etc.)
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        // Clear settings file if exists
        let settings_file = app_data_dir.join("settings.json");
        if settings_file.exists() {
            let _ = std::fs::remove_file(&settings_file);
            println!("[clear_all_data] Removed settings file");
        }

        // Clear any other cache files
        let cache_dir = app_data_dir.join("cache");
        if cache_dir.exists() {
            let _ = std::fs::remove_dir_all(&cache_dir);
            println!("[clear_all_data] Removed cache directory");
        }

        // Clear localStorage data (stored by Tauri)
        let local_storage = app_data_dir.join("Local Storage");
        if local_storage.exists() {
            let _ = std::fs::remove_dir_all(&local_storage);
            println!("[clear_all_data] Removed localStorage");
        }

        println!("[clear_all_data] App data directory cleaned");
    }

    println!("[clear_all_data] Complete! All data cleared successfully.");
    Ok(())
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
// ============================================================================
// Torrent Integration
// ============================================================================

#[tauri::command]
async fn init_torrent_backend(
    download_dir: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!(
        "[Torrent Backend] Initializing with download_dir: {}",
        download_dir
    );
    let dir = std::path::PathBuf::from(&download_dir);

    // Try to create directory with better error message
    if !dir.exists() {
        println!("[Torrent Backend] Creating directory: {:?}", dir);
        std::fs::create_dir_all(&dir).map_err(|e| {
            let err_msg = format!(
                "Failed to create directory '{}': {} (error code: {:?})",
                download_dir,
                e,
                e.kind()
            );
            eprintln!("[Torrent Backend] {}", err_msg);
            err_msg
        })?;
    }

    // Verify we can write to the directory
    let test_file = dir.join(".test_write");
    if let Err(e) = std::fs::write(&test_file, b"test") {
        let err_msg = format!(
            "Cannot write to directory '{}': {} (error code: {:?})",
            download_dir,
            e,
            e.kind()
        );
        eprintln!("[Torrent Backend] {}", err_msg);
        return Err(err_msg);
    }
    let _ = std::fs::remove_file(test_file);

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

#[tauri::command]
fn set_speed(value: f32, state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        player.set_speed(value)
    } else {
        Ok(())
    }
}

#[tauri::command]
fn set_reverb(mix: f32, decay: f32, state: State<AppState>) -> Result<(), String> {
    let player_guard = state.player.lock().unwrap();
    if let Some(ref player) = *player_guard {
        player.set_reverb(mix, decay)
    } else {
        Ok(())
    }
}

// ============================================================================
// Mobile Companion Server Commands
// ============================================================================

#[tauri::command]
async fn start_mobile_server(
    state: State<'_, AppState>,
    _app_handle: AppHandle,
) -> Result<(), String> {
    // Check if already running
    {
        let running = state
            .server_running
            .lock()
            .map_err(|_| "Failed to lock server_running".to_string())?;
        if *running {
            return Ok(());
        }
    }

    // Mark as running
    {
        let mut running = state
            .server_running
            .lock()
            .map_err(|_| "Failed to lock server_running".to_string())?;
        *running = true;
    }

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Store shutdown sender
    {
        let mut tx_guard = state
            .server_shutdown_tx
            .lock()
            .map_err(|_| "Failed to lock server_shutdown_tx".to_string())?;
        *tx_guard = Some(shutdown_tx);
    }

    // Start server in background with the real app handle
    let config = server::ServerConfig::default();
    let port = config.port;
    let server_running = state.server_running.clone();
    let app_handle_clone = _app_handle.clone();

    tokio::spawn(async move {
        if let Err(e) = server::start_server(app_handle_clone, config, shutdown_rx).await {
            eprintln!("[Server] Failed to start: {}", e);
            if let Ok(mut running) = server_running.lock() {
                *running = false;
            }
        }
    });

    println!("[Server] Mobile companion server started on port {}", port);
    Ok(())
}

#[tauri::command]
async fn stop_mobile_server(state: State<'_, AppState>) -> Result<(), String> {
    // Send shutdown signal
    {
        let tx_guard = state
            .server_shutdown_tx
            .lock()
            .map_err(|_| "Failed to lock server_shutdown_tx".to_string())?;
        if let Some(ref tx) = *tx_guard {
            let _ = tx.send(());
        }
    }

    // Mark as not running
    let mut running = state
        .server_running
        .lock()
        .map_err(|_| "Failed to lock server_running".to_string())?;
    *running = false;
    println!("[Server] Mobile companion server stopped");
    Ok(())
}

#[tauri::command]
async fn get_server_status(state: State<'_, AppState>) -> Result<bool, String> {
    let running = state
        .server_running
        .lock()
        .map_err(|_| "Failed to lock server_running".to_string())?;
    Ok(*running)
}

#[tauri::command]
async fn get_p2p_peers(
    state: State<'_, AppState>,
) -> Result<Vec<p2p::discovery::DiscoveredPeer>, String> {
    let p2p_guard = state.p2p_manager.read().await;
    if let Some(ref p2p) = *p2p_guard {
        Ok(p2p.get_peers().await)
    } else {
        Ok(vec![])
    }
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
            get_lyrics,
            get_cached_lyrics,
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
            start_mobile_server,
            stop_mobile_server,
            get_server_status,
            get_p2p_peers,
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
            set_speed,
            set_reverb,
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

            // Start mobile companion server and P2P in background
            let app_handle = _app.handle().clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async {
                    // Initialize P2P manager
                    let device_name = p2p::get_device_name();
                    match P2PManager::new(device_name).await {
                        Ok(p2p) => {
                            println!("[P2P] Manager initialized successfully");
                            let state = app_handle.state::<AppState>();
                            let mut p2p_guard = state.p2p_manager.write().await;
                            *p2p_guard = Some(p2p);
                        }
                        Err(e) => {
                            eprintln!("[P2P] Failed to initialize: {}", e);
                        }
                    }
                });
            });

            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            println!("Second instance launched");
        }))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
