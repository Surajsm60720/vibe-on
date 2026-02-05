# VIBE-ON Server Code Analysis: Cover Art & Mobile Streaming

## Overview
This document details the server-side implementation of:
1. Album art serving via `/cover/` endpoint
2. Audio streaming for mobile playback
3. Mobile playback control via WebSocket handlers
4. Error handling and logging

---

## 1. COVER ART ENDPOINT (`/cover/*path`)

### Route Registration
**File:** [src-tauri/src/server/mod.rs](src-tauri/src/server/mod.rs#L230)
```rust
.route("/cover/*path", get(get_cover))
```

### Implementation: `get_cover()` Handler
**File:** [src-tauri/src/server/routes.rs](src-tauri/src/server/routes.rs#L584-L650)

```rust
pub async fn get_cover(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    let track_path = urlencoding::decode(&path)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_string();
    
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
```

### Cover Extraction from Audio Files
**File:** [src-tauri/src/server/routes.rs](src-tauri/src/server/routes.rs#L650-L673)

```rust
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
```

### Cover Serving Strategy

**Dual Fallback Approach:**
1. **Primary:** Checks cached cover files in the covers directory
   - Looks up track in database to find associated cover file
   - Returns cached PNG or JPEG
   - Fast, pre-extracted covers

2. **Fallback:** Extracts cover from audio file metadata
   - Uses `lofty` library to read embedded tags (ID3, MP4, etc.)
   - Extracts first embedded picture
   - Supports: PNG, JPEG, GIF, BMP formats

**Headers:**
- `Content-Type`: `image/png` or `image/jpeg` (or detected format)
- `Cache-Control`: `public, max-age=86400` (24-hour client-side caching)

**Error Handling:**
- `400 Bad Request`: Invalid URL path encoding
- `500 Internal Server Error`: Database lock failure
- `404 Not Found`: No cover found in either cache or file metadata

---

## 2. AUDIO STREAMING ENDPOINTS

### Route Registration
**File:** [src-tauri/src/server/mod.rs](src-tauri/src/server/mod.rs#L231-L232)
```rust
// Audio streaming
.route("/stream/:path", get(stream_audio_file))
.route("/stream", get(stream_audio))
```

### Implementation: `stream_audio_file()` - Direct File Streaming
**File:** [src-tauri/src/server/routes.rs](src-tauri/src/server/routes.rs#L673-L723)

```rust
/// Stream audio to mobile client from a specific file path
/// Supports HTTP Range requests for seeking
pub async fn stream_audio_file(
    Path(encoded_path): Path<String>,
) -> Result<Response, StatusCode> {
    
    // Decode the path
    let track_path = urlencoding::decode(&encoded_path)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_string();
    
    // Validate path exists and is accessible
    if !std::path::Path::new(&track_path).exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Read file metadata for size
    let file_metadata = tokio::fs::metadata(&track_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let file_size = file_metadata.len();
    
    // Read the entire file
    let data = tokio::fs::read(&track_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
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
    
    // Build response with proper headers for streaming
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_size.to_string())
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=604800")
        .body(Body::from(data))
        .unwrap())
}
```

### Implementation: `stream_audio()` - Current Track Streaming
**File:** [src-tauri/src/server/routes.rs](src-tauri/src/server/routes.rs#L723-L775)

```rust
/// Stream audio to mobile client (legacy endpoint for current track)
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
```

### Query Parameters
**File:** [src-tauri/src/server/routes.rs](src-tauri/src/server/routes.rs#L165-L169)

```rust
/// Stream query params
#[derive(Debug, Deserialize)]
pub struct StreamParams {
    pub start: Option<u64>,
}
```

### Supported Audio Formats

| Format | Codec | MIME Type | Notes |
|--------|-------|-----------|-------|
| FLAC | FLAC (Lossless) | `audio/flac` | Highest quality, native support |
| MP3 | MPEG-1 Layer III | `audio/mpeg` | Widely compatible |
| M4A/AAC | Advanced Audio Codec | `audio/aac` | iTunes/Apple format |
| OGG/Opus | Opus/Vorbis | `audio/ogg` | Open source formats |
| WAV | PCM | `audio/wav` | Uncompressed, large files |

### Streaming Headers
- `Content-Type`: Detected from file extension
- `Content-Length`: File size in bytes
- `Accept-Ranges`: `bytes` (enables HTTP range requests for seeking)
- `Cache-Control`: `public, max-age=604800` (7-day browser caching)
- `X-Audio-Format`: Custom header with content type
- `X-Audio-Start-Sample`: Sample offset for seeking

**Limitations:**
1. **Entire file loaded into memory**: `tokio::fs::read()` loads the whole file
   - No streaming chunks, not optimal for large files
   - Mobile must wait for full file before seeking
   
2. **Sample calculation not implemented**: `start_sample` parameter is parsed but not used to seek within file

3. **No byte range handling**: Setup for HTTP `Range` requests but not actually implemented

---

## 3. WEBSOCKET HANDLERS: Mobile Playback Control

### Message Types
**File:** [src-tauri/src/server/websocket.rs](src-tauri/src/server/websocket.rs#L20-L72)

```rust
pub enum ClientMessage {
    // ... other commands ...
    /// Request audio streaming to mobile
    RequestStreamToMobile,
    /// Stop streaming to mobile
    StopStreamToMobile,
    /// Mobile is ready to receive stream
    HandoffReady,
    /// Start mobile playback (client-initiated)
    StartMobilePlayback,
    /// Stop mobile playback (client-initiated)
    StopMobilePlayback,
    /// Mobile playback position update
    MobilePositionUpdate { position_secs: f64 },
    // ... other commands ...
}
```

### Server Response Messages
**File:** [src-tauri/src/server/websocket.rs](src-tauri/src/server/websocket.rs#L74-L180)

```rust
pub enum ServerMessage {
    // ... other messages ...
    /// Prepare for audio handoff
    #[serde(rename_all = "camelCase")]
    HandoffPrepare {
        sample: u64,
        url: String,
    },
    /// Commit handoff (start playing)
    HandoffCommit,
    /// Stream stopped
    StreamStopped,
    /// Error message
    #[serde(rename = "Error")]
    Error { message: String },
    // ... other messages ...
}
```

### Handler: `StartMobilePlayback`
**File:** [src-tauri/src/server/websocket.rs](src-tauri/src/server/websocket.rs#L717-L761)

```rust
ClientMessage::StartMobilePlayback => {
    log::info!("📱 StartMobilePlayback command from mobile ({})", client_id);
    
    // Pause PC playback
    if let Ok(mut player_guard) = app_state.player.lock() {
        if let Some(ref mut player) = *player_guard {
            player.pause();
            log::info!("⏸️ PC playback paused for mobile streaming");
        }
    }
    
    // Get current track info and send stream URL to mobile
    let (track_path, position, stream_url) = {
        let player_guard = app_state.player.lock().ok();
        match player_guard.as_ref().and_then(|p| p.as_ref()) {
            Some(player) => {
                let status = player.get_status();
                match status.track {
                    Some(track) => {
                        let position = status.position_secs;
                        let local_ip = local_ip().unwrap_or("127.0.0.1".to_string());
                        let port = state.config.port;
                        let encoded_path = urlencoding::encode(&track.path).to_string();
                        let url = format!("http://{}:{}/stream/{}", local_ip, port, encoded_path);
                        (Some(track.path), position, url)
                    }
                    None => (None, 0.0, String::new())
                }
            }
            None => (None, 0.0, String::new())
        }
    };
    
    if track_path.is_some() {
        log::info!("🎵 Sending stream URL to mobile: {}", stream_url);
        let _ = reply_tx.send(ServerMessage::HandoffPrepare { 
            sample: (position * 44100.0) as u64,
            url: stream_url 
        }).await;
        
        // Notify frontend that output changed
        let _ = state.app_handle.emit("output-changed", serde_json::json!({
            "output": "mobile"
        }));
    } else {
        let _ = reply_tx.send(ServerMessage::Error {
            message: "No track currently playing".to_string(),
        }).await;
    }
}
```

**Logic Flow:**
1. **Log the command** with client ID
2. **Pause PC playback** to avoid dual playback
3. **Lock player state** and extract:
   - Current track path
   - Current playback position
4. **Build stream URL**: `http://<local_ip>:5000/stream/<encoded_path>`
5. **Calculate sample position**: `position_secs * 44100.0` (assumes 44.1kHz sample rate)
6. **Send `HandoffPrepare`** with URL and sample offset
7. **Emit frontend event**: `output-changed` with `{"output": "mobile"}`
8. **Error case**: If no track playing, send error message

### Handler: `StopMobilePlayback`
**File:** [src-tauri/src/server/websocket.rs](src-tauri/src/server/websocket.rs#L767-L781)

```rust
ClientMessage::StopMobilePlayback => {
    log::info!("📱 StopMobilePlayback command from mobile ({})", client_id);
    
    // Resume desktop playback
    if let Ok(mut player_guard) = app_state.player.lock() {
        if let Some(ref mut player) = *player_guard {
            player.resume();
            log::info!("▶️ PC playback resumed");
        }
    }
    
    state.broadcast(ServerEvent::StreamStopped);
    
    // Notify frontend that output changed
    let _ = state.app_handle.emit("output-changed", serde_json::json!({
        "output": "desktop"
    }));
}
```

**Logic Flow:**
1. **Log the command** with client ID
2. **Resume PC playback**
3. **Broadcast `StreamStopped`** event to all clients
4. **Emit frontend event**: `output-changed` with `{"output": "desktop"}`

---

## 4. ERROR HANDLING & LOGGING

### Logging Strategy
The server uses comprehensive logging with emoji prefixes for quick identification:

**Mobile-related logs:**
```rust
log::info!("📱 StartMobilePlayback command from mobile ({})", client_id);
log::info!("⏸️ PC playback paused for mobile streaming");
log::info!("▶️ PC playback resumed");
log::info!("🎵 Sending stream URL to mobile: {}", stream_url);
```

**Connection logs:**
```rust
log::info!("🔌 New WebSocket connection established");
log::info!("📱 New WebSocket connection accepted (ID: {})", client_id);
log::info!("Mobile client disconnected: {} ({})", client.name, client.id);
```

**Error logs:**
```rust
log::warn!("❌ Invalid WebSocket message: {}", e);
log::warn!("❌ Failed to send reply to client");
log::error!("❌ Failed to emit mobile_client_connected: {}", e);
```

### HTTP Error Responses

#### `/cover/*path` Endpoint:
- **400 Bad Request**: URL encoding errors in path
- **404 Not Found**: No cover found in cache or file metadata
- **500 Internal Server Error**: Database lock failure

#### `/stream/:path` Endpoint:
- **400 Bad Request**: Invalid URL encoding
- **404 Not Found**: File doesn't exist

#### `/stream` Endpoint:
- **404 Not Found**: No current track playing
- **500 Internal Server Error**: Player lock failure
- **503 Service Unavailable**: Player not initialized

### WebSocket Error Handling:

**Invalid message parsing:**
```rust
log::warn!("❌ Invalid WebSocket message: {}", e);
state.broadcast(ServerEvent::Error {
    message: format!("Invalid message: {}", e),
});
```

**No track playing error:**
```rust
let _ = reply_tx.send(ServerMessage::Error {
    message: "No track currently playing".to_string(),
}).await;
```

**Failed to send reply:**
```rust
log::warn!("❌ Failed to send reply to client");
```

### Graceful Degradation:
- Lock failures use `map_err()` and `ok()` to prevent panics
- Missing player is handled as `ServiceUnavailable` (503)
- Disconnected clients are cleanly removed from connection list
- Failed event emissions are logged but don't crash the handler

---

## 5. PROTOCOL & CONFIGURATION

### mDNS Advertisement
**File:** [src-tauri/src/server/mod.rs](src-tauri/src/server/mod.rs#L294-L310)

The server advertises itself via mDNS with:
- **Service name**: User device name
- **Port**: Default 5000 (configurable)
- **Type**: Allows mobile clients to auto-discover

### Server Configuration
**File:** [src-tauri/src/server/mod.rs](src-tauri/src/server/mod.rs#L29-L42)

```rust
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,
    /// Server name for mDNS
    pub server_name: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 5000,
            server_name: crate::p2p::get_device_name(),
        }
    }
}
```

### CORS Policy
**File:** [src-tauri/src/server/mod.rs](src-tauri/src/server/mod.rs#L246-L250)

```rust
.layer(
    CorsLayer::new()
        .allow_origin(Any)  // Allow requests from any origin
        .allow_methods(Any) // Allow all HTTP methods
        .allow_headers(Any) // Allow all headers
)
```

---

## 6. CORE LIBRARY (vibe-on-core)

### P2P Streaming Protocol
**File:** [vibe-on-core/src/protocol.rs](vibe-on-core/src/protocol.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamRequest {
    RequestTrack {
        track_path: String,
        start_byte: u64,
    },
    Seek { byte_offset: u64 },
    Stop,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamResponse {
    Header {
        format: String,
        sample_rate: u32,
        channels: u16,
        duration_secs: f64,
        file_size: u64,
        title: String,
        artist: String,
        album: String,
    },
    Chunk {
        sequence: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    SeekAck { byte_offset: u64 },
    Stopped,
    Pong,
    Error { message: String },
}
```

This P2P protocol supports:
- **Chunked streaming** with sequence numbers for reliability
- **Seeking** via byte offset
- **Metadata exchange** (format, sample rate, channels, duration, title, artist, album)
- **Error handling** with error messages
- **Keepalive** via Ping/Pong

Uses **CBOR serialization** for compact binary encoding with up to 100MB response buffers.

---

## 7. SUMMARY TABLE

| Feature | Endpoint | Format | Caching | Limitations |
|---------|----------|--------|---------|-------------|
| **Cover Art** | `/cover/*path` | PNG/JPEG/GIF/BMP | 24h (86400s) | Extracted from metadata, no CDN |
| **Audio Stream (File)** | `/stream/:path` | FLAC/MP3/AAC/OGG/WAV | 7d (604800s) | Full file in memory, no chunking |
| **Audio Stream (Current)** | `/stream?start=<sample>` | Detected from extension | 7d | `start_sample` not implemented |
| **Mobile Playback** | WebSocket `StartMobilePlayback` | N/A | N/A | Assumes 44.1kHz, hardcoded sample rate |
| **Playback Stop** | WebSocket `StopMobilePlayback` | N/A | N/A | Simple resume, no state sync |

---

## 8. KEY IMPLEMENTATION NOTES

### Assumptions
1. **Sample rate**: Hardcoded to 44100 Hz in sample calculation
2. **Desktop IP**: Resolved locally; fallback to `127.0.0.1` for remote access issues
3. **File accessibility**: No sandboxing; server can access any file it has permissions for
4. **Memory efficiency**: Entire audio files loaded into memory during streaming

### Thread Safety
- Uses `Arc<ServerState>` for shared state
- `RwLock` for player access
- `Mutex` for database access
- Broadcast channel for event distribution

### Future Improvements
- [ ] Implement HTTP Range request seeking
- [ ] Add chunked/streaming response bodies
- [ ] Dynamic sample rate detection from file metadata
- [ ] P2P protocol migration (partial implementation in vibe-on-core)
- [ ] Bandwidth-aware format selection
- [ ] Direct audio file seeking without loading entire file
