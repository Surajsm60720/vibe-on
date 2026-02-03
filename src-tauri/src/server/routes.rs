//! HTTP REST API routes for VIBE-ON! server

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ServerState, TrackSummary};

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Server info response
#[derive(Serialize)]
pub struct ServerInfoResponse {
    pub name: String,
    pub version: String,
    pub platform: String,
    #[serde(rename = "librarySize")]
    pub library_size: usize,
    pub port: u16,
    #[serde(rename = "localIp")]
    pub local_ip: Option<String>,
}

/// Playback state response
#[derive(Serialize)]
pub struct PlaybackStateResponse {
    #[serde(rename = "isPlaying")]
    pub is_playing: bool,
    #[serde(rename = "currentTrack")]
    pub current_track: Option<TrackDetail>,
    #[serde(rename = "positionSecs")]
    pub position_secs: f64,
    #[serde(rename = "durationSecs")]
    pub duration_secs: f64,
    pub volume: f64,
    pub shuffle: bool,
    #[serde(rename = "repeatMode")]
    pub repeat_mode: String,
    pub queue: Vec<TrackSummary>,
}

/// Detailed track info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDetail {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(rename = "durationSecs")]
    pub duration_secs: f64,
    #[serde(rename = "discNumber")]
    pub disc_number: Option<u32>,
    #[serde(rename = "trackNumber")]
    pub track_number: Option<u32>,
    #[serde(rename = "coverUrl")]
    pub cover_url: Option<String>,
}

/// Album info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumInfo {
    pub name: String,
    pub artist: String,
    #[serde(rename = "coverUrl")]
    pub cover_url: Option<String>,
    #[serde(rename = "trackCount")]
    pub track_count: usize,
}

/// Artist info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistInfo {
    pub name: String,
    #[serde(rename = "albumCount")]
    pub album_count: usize,
    #[serde(rename = "trackCount")]
    pub track_count: usize,
}

/// Library response
#[derive(Serialize)]
pub struct LibraryResponse {
    pub tracks: Vec<TrackDetail>,
    pub total: usize,
}

/// Search response
#[derive(Serialize)]
pub struct SearchResponse {
    pub tracks: Vec<TrackDetail>,
    pub albums: Vec<AlbumInfo>,
    pub artists: Vec<ArtistInfo>,
}

/// Albums response
#[derive(Serialize)]
pub struct AlbumsResponse {
    pub albums: Vec<AlbumInfo>,
    pub total: usize,
}

/// Album detail response
#[derive(Serialize)]
pub struct AlbumDetailResponse {
    pub album: AlbumInfo,
    pub tracks: Vec<TrackDetail>,
}

/// Artists response
#[derive(Serialize)]
pub struct ArtistsResponse {
    pub artists: Vec<ArtistInfo>,
    pub total: usize,
}

/// Artist detail response
#[derive(Serialize)]
pub struct ArtistDetailResponse {
    pub artist: ArtistInfo,
    pub albums: Vec<AlbumInfo>,
    pub tracks: Vec<TrackDetail>,
}

/// Lyrics response
#[derive(Serialize)]
pub struct LyricsResponse {
    #[serde(rename = "trackPath")]
    pub track_path: String,
    #[serde(rename = "hasSynced")]
    pub has_synced: bool,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    pub instrumental: bool,
}

/// Pagination query params
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// Search query params
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// Stream query params
#[derive(Debug, Deserialize)]
pub struct StreamParams {
    pub start: Option<u64>,
}

/// Health check endpoint
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// Get server info
pub async fn get_server_info(
    State(state): State<Arc<ServerState>>,
) -> Json<ServerInfoResponse> {
    let app_state = state.app_state();
    let library_size = app_state.db.lock().ok()
        .and_then(|db| db.as_ref().map(|d| d.get_all_tracks().map(|t| t.len()).unwrap_or(0)))
        .unwrap_or(0);
    
    // Get local IP address
    let local_ip = get_local_ip();
    
    Json(ServerInfoResponse {
        name: state.config.server_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        library_size,
        port: state.config.port,
        local_ip,
    })
}

/// Get local IP address for LAN connections
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    
    // Create a UDP socket and "connect" to a public IP (doesn't actually send data)
    // This lets us determine which local interface would be used
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip().to_string())
}

/// Get current playback state
pub async fn get_playback_state(
    State(state): State<Arc<ServerState>>,
) -> Json<PlaybackStateResponse> {
    let app_state = state.app_state();
    
    // Get player status
    let (is_playing, current_track, position_secs, duration_secs, volume) = {
        if let Ok(player_guard) = app_state.player.lock() {
            if let Some(ref player) = *player_guard {
                let status = player.get_status();
                let is_playing = status.state == crate::audio::PlayerState::Playing;
                let position = status.position_secs;
                let volume = status.volume;
                let duration = status.track.as_ref().map(|t| t.duration_secs).unwrap_or(0.0);
                
                let track = status.track.map(|t| TrackDetail {
                    path: t.path.clone(),
                    title: t.title.clone(),
                    artist: t.artist.clone(),
                    album: t.album.clone(),
                    duration_secs: t.duration_secs,
                    disc_number: None,
                    track_number: None,
                    cover_url: Some(format!("/cover/{}", urlencoding::encode(&t.path))),
                });
                
                (is_playing, track, position, duration, volume)
            } else {
                (false, None, 0.0, 0.0, 1.0)
            }
        } else {
            (false, None, 0.0, 0.0, 1.0)
        }
    };
    
    Json(PlaybackStateResponse {
        is_playing,
        current_track,
        position_secs,
        duration_secs,
        volume: volume as f64,
        shuffle: false, // TODO: Get from frontend state
        repeat_mode: "off".to_string(),
        queue: vec![], // TODO: Implement queue sync
    })
}

/// Get library tracks
pub async fn get_library(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<LibraryResponse>, StatusCode> {
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50);
    
    let app_state = state.app_state();
    let tracks = app_state.db.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .get_all_tracks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = tracks.len();
    let tracks: Vec<TrackDetail> = tracks
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|t| TrackDetail {
            path: t.path.clone(),
            title: t.title,
            artist: t.artist,
            album: t.album,
            duration_secs: t.duration_secs,
            disc_number: t.disc_number,
            track_number: t.track_number,
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&t.path))),
        })
        .collect();
    
    Ok(Json(LibraryResponse { tracks, total }))
}

/// Search library
pub async fn search_library(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let query = params.q.to_lowercase();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50);
    
    let app_state = state.app_state();
    let all_tracks = app_state.db.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .get_all_tracks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Filter tracks
    let tracks: Vec<TrackDetail> = all_tracks
        .iter()
        .filter(|t| {
            t.title.to_lowercase().contains(&query) ||
            t.artist.to_lowercase().contains(&query) ||
            t.album.to_lowercase().contains(&query)
        })
        .skip(offset)
        .take(limit)
        .map(|t| TrackDetail {
            path: t.path.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            duration_secs: t.duration_secs,
            disc_number: t.disc_number,
            track_number: t.track_number,
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&t.path))),
        })
        .collect();
    
    // Get unique albums
    let mut albums_map = std::collections::HashMap::new();
    for track in &all_tracks {
        if track.album.to_lowercase().contains(&query) ||
           track.artist.to_lowercase().contains(&query) {
            let key = (track.album.clone(), track.artist.clone());
            let entry = albums_map.entry(key).or_insert((0, track.path.clone()));
            entry.0 += 1;
        }
    }
    let albums: Vec<AlbumInfo> = albums_map
        .into_iter()
        .take(limit)
        .map(|((name, artist), (count, path))| AlbumInfo {
            name,
            artist,
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&path))),
            track_count: count,
        })
        .collect();
    
    // Get unique artists
    let mut artists_map = std::collections::HashMap::new();
    for track in &all_tracks {
        if track.artist.to_lowercase().contains(&query) {
            let entry = artists_map.entry(track.artist.clone()).or_insert((std::collections::HashSet::new(), 0));
            entry.0.insert(track.album.clone());
            entry.1 += 1;
        }
    }
    let artists: Vec<ArtistInfo> = artists_map
        .into_iter()
        .take(limit)
        .map(|(name, (albums, track_count))| ArtistInfo {
            name,
            album_count: albums.len(),
            track_count,
        })
        .collect();
    
    Ok(Json(SearchResponse { tracks, albums, artists }))
}

/// Get all albums
pub async fn get_albums(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<AlbumsResponse>, StatusCode> {
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50);
    
    let app_state = state.app_state();
    let all_tracks = app_state.db.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .get_all_tracks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Group by album
    let mut albums_map = std::collections::HashMap::new();
    for track in &all_tracks {
        let key = (track.album.clone(), track.artist.clone());
        let entry = albums_map.entry(key).or_insert((0, track.path.clone()));
        entry.0 += 1;
    }
    
    let total = albums_map.len();
    let albums: Vec<AlbumInfo> = albums_map
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|((name, artist), (count, path))| AlbumInfo {
            name,
            artist,
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&path))),
            track_count: count,
        })
        .collect();
    
    Ok(Json(AlbumsResponse { albums, total }))
}

/// Get album detail
pub async fn get_album_detail(
    State(state): State<Arc<ServerState>>,
    Path((name, artist)): Path<(String, String)>,
) -> Result<Json<AlbumDetailResponse>, StatusCode> {
    let name = urlencoding::decode(&name).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    let artist = urlencoding::decode(&artist).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    let app_state = state.app_state();
    let all_tracks = app_state.db.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .get_all_tracks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let tracks: Vec<TrackDetail> = all_tracks
        .into_iter()
        .filter(|t| t.album == name && t.artist == artist)
        .map(|t| TrackDetail {
            path: t.path.clone(),
            title: t.title,
            artist: t.artist,
            album: t.album,
            duration_secs: t.duration_secs,
            disc_number: t.disc_number,
            track_number: t.track_number,
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&t.path))),
        })
        .collect();
    
    if tracks.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    let album = AlbumInfo {
        name: name.clone(),
        artist: artist.clone(),
        cover_url: tracks.first().and_then(|t| t.cover_url.clone()),
        track_count: tracks.len(),
    };
    
    Ok(Json(AlbumDetailResponse { album, tracks }))
}

/// Get all artists
pub async fn get_artists(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ArtistsResponse>, StatusCode> {
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50);
    
    let app_state = state.app_state();
    let all_tracks = app_state.db.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .get_all_tracks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Group by artist
    let mut artists_map = std::collections::HashMap::new();
    for track in &all_tracks {
        let entry = artists_map.entry(track.artist.clone()).or_insert((std::collections::HashSet::new(), 0));
        entry.0.insert(track.album.clone());
        entry.1 += 1;
    }
    
    let total = artists_map.len();
    let artists: Vec<ArtistInfo> = artists_map
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(name, (albums, track_count))| ArtistInfo {
            name,
            album_count: albums.len(),
            track_count,
        })
        .collect();
    
    Ok(Json(ArtistsResponse { artists, total }))
}

/// Get artist detail
pub async fn get_artist_detail(
    State(state): State<Arc<ServerState>>,
    Path(name): Path<String>,
) -> Result<Json<ArtistDetailResponse>, StatusCode> {
    let name = urlencoding::decode(&name).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    let app_state = state.app_state();
    let all_tracks = app_state.db.lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .get_all_tracks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let tracks: Vec<TrackDetail> = all_tracks
        .iter()
        .filter(|t| t.artist == name)
        .map(|t| TrackDetail {
            path: t.path.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            duration_secs: t.duration_secs,
            disc_number: t.disc_number,
            track_number: t.track_number,
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&t.path))),
        })
        .collect();
    
    if tracks.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Get unique albums
    let mut albums_map = std::collections::HashMap::new();
    for track in &tracks {
        let entry = albums_map.entry(track.album.clone()).or_insert((0, track.path.clone()));
        entry.0 += 1;
    }
    let albums: Vec<AlbumInfo> = albums_map
        .into_iter()
        .map(|(album_name, (count, path))| AlbumInfo {
            name: album_name,
            artist: name.clone(),
            cover_url: Some(format!("/cover/{}", urlencoding::encode(&path))),
            track_count: count,
        })
        .collect();
    
    let artist = ArtistInfo {
        name: name.clone(),
        album_count: albums.len(),
        track_count: tracks.len(),
    };
    
    Ok(Json(ArtistDetailResponse { artist, albums, tracks }))
}

/// Get lyrics for a track
pub async fn get_lyrics(
    State(_state): State<Arc<ServerState>>,
    Path(path): Path<String>,
) -> Result<Json<LyricsResponse>, StatusCode> {
    let track_path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    // TODO: Implement lyrics fetching from cache/API
    Ok(Json(LyricsResponse {
        track_path,
        has_synced: false,
        synced_lyrics: None,
        plain_lyrics: None,
        instrumental: false,
    }))
}

/// Get cover art for a track
pub async fn get_cover(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    let track_path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    // Try to get cover from covers directory by looking up track info
    let app_state = state.app_state();
    let cover_file_path = {
        let db_guard = app_state.db.lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        if let Some(ref db) = *db_guard {
            let covers_dir = db.get_covers_dir();
            
            if let Ok(tracks) = db.get_all_tracks() {
                if let Some(track) = tracks.iter().find(|t| t.path == track_path) {
                    if let Some(ref cover_filename) = track.cover_image {
                        Some(covers_dir.join(cover_filename))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };
    
    // Try to read from cover file (after releasing lock)
    if let Some(cover_path) = cover_file_path {
        if let Ok(data) = tokio::fs::read(&cover_path).await {
            let content_type = if cover_path.to_string_lossy().ends_with(".png") {
                "image/png"
            } else {
                "image/jpeg"
            };
            
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(Body::from(data))
                .unwrap());
        }
    }
    
    // Try to extract from audio file
    match extract_cover_from_file(&track_path) {
        Some((data, mime)) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(Body::from(data))
                .unwrap())
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Extract cover art from an audio file
fn extract_cover_from_file(path: &str) -> Option<(Vec<u8>, &'static str)> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    
    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag())?;
    
    for picture in tag.pictures() {
        let mime = match picture.mime_type() {
            Some(lofty::picture::MimeType::Png) => "image/png",
            Some(lofty::picture::MimeType::Jpeg) => "image/jpeg",
            Some(lofty::picture::MimeType::Gif) => "image/gif",
            Some(lofty::picture::MimeType::Bmp) => "image/bmp",
            _ => "image/jpeg",
        };
        return Some((picture.data().to_vec(), mime));
    }
    
    None
}

/// Stream audio to mobile client
pub async fn stream_audio(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    let start_sample = params.start.unwrap_or(0);
    
    // Get current track
    let app_state = state.app_state();
    let track_path = {
        let player_guard = app_state.player.lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let player = player_guard.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let status = player.get_status();
        status.track.map(|t| t.path).ok_or(StatusCode::NOT_FOUND)?
    };
    
    // Read the audio file
    let data = tokio::fs::read(&track_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    // Skip to start position if needed
    let data = if start_sample > 0 {
        // For raw streaming, we'd calculate byte offset from sample
        // For now, send the full file (mobile will seek)
        data
    } else {
        data
    };
    
    // Determine content type from extension
    let content_type = if track_path.ends_with(".flac") {
        "audio/flac"
    } else if track_path.ends_with(".mp3") {
        "audio/mpeg"
    } else if track_path.ends_with(".m4a") || track_path.ends_with(".aac") {
        "audio/aac"
    } else if track_path.ends_with(".ogg") || track_path.ends_with(".opus") {
        "audio/ogg"
    } else if track_path.ends_with(".wav") {
        "audio/wav"
    } else {
        "application/octet-stream"
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header("X-Audio-Format", content_type)
        .header("X-Audio-Start-Sample", start_sample.to_string())
        .body(Body::from(data))
        .unwrap())
}
