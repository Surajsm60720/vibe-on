# VIBE-ON! Mobile Android App - Technical Specification

## Overview
Build a native Android companion app for VIBE-ON! desktop music player that connects via local network to stream music with zero quality loss using P2P technology.

---

## Core Architecture

### Connection Methods (in priority order)
1. **P2P QUIC Stream** (Primary) - Direct lossless audio streaming via libp2p
2. **HTTP REST API** - Library browsing, metadata, cover art
3. **WebSocket** - Real-time playback control, state sync, WebRTC signaling

### Desktop Server Endpoints (Already Implemented)
```
Base URL: http://{desktop-ip}:5443

REST API:
- GET  /health
- GET  /api/info
- GET  /api/playback
- GET  /api/library?offset=0&limit=50
- GET  /api/albums
- GET  /api/artists
- GET  /api/lyrics/{track_path}
- GET  /cover/{track_path}
- GET  /stream?start={sample_position}

WebSocket:
- WS   /ws
```

---

## Technical Requirements

### 1. Network Discovery (mDNS)
- Auto-discover VIBE-ON! desktop server on LAN using mDNS service `_vibe-on._tcp`
- Display list of discovered devices with hostname and IP
- Allow manual IP entry for networks without mDNS
- Show connection status indicator

### 2. P2P Streaming Architecture

#### Desktop Server Implementation (Reference)
The desktop already implements:
- **Protocol**: `/vibe-on/stream/1.0.0` over QUIC
- **Transport**: libp2p with noise encryption, yamux multiplexing
- **Codec**: CBOR serialization
- **Buffering**: Files ≤20MB pre-buffered entirely, >20MB uses 3-second ring buffer
- **Format**: Original file bytes (FLAC, MP3, AAC, etc.) - no re-encoding

#### Mobile Requirements
```kotlin
// Connection flow:
1. Discover desktop via mDNS
2. Connect WebSocket to get peer info
3. Send WebRTC offer via WebSocket (signaling relay)
4. Desktop responds with WebRTC answer
5. Establish P2P QUIC connection via libp2p
6. Request track stream with start position
7. Receive StreamResponse with header (format, sample_rate, channels, duration)
8. Receive audio chunks (sequence, data bytes, is_last flag)
9. Feed to platform decoder (MediaCodec)
```

#### P2P Protocol Messages (CBOR)
```rust
// Request types
StreamRequest::RequestTrack { track_path, start_byte }
StreamRequest::Seek { byte_offset }
StreamRequest::Stop
StreamRequest::Ping

// Response types
StreamResponse::Header { format, sample_rate, channels, duration_secs, file_size, title, artist, album }
StreamResponse::Chunk { sequence, data: Vec<u8>, is_last }
StreamResponse::SeekAck { byte_offset }
StreamResponse::Stopped
StreamResponse::Pong
StreamResponse::Error { message }
```

### 3. Audio Playback Engine

#### Requirements
- **Decoder**: Use Android MediaCodec (hardware-accelerated, supports FLAC/MP3/AAC/OGG/Opus)
- **Buffering**: Adaptive buffer (3-second minimum for seamless playback)
- **Seeking**: Instant seek for pre-buffered tracks, request new stream for large files
- **Gapless**: Support gapless playback between tracks
- **Background**: Continue playback when app backgrounded
- **Lock Screen**: Show media controls with cover art

#### Implementation Pattern
```kotlin
class P2PAudioPlayer {
    private val mediaCodec: MediaCodec
    private val audioTrack: AudioTrack
    private val buffer: AdaptiveBuffer
    
    fun playTrack(trackPath: String, startPosition: Long) {
        // 1. Send StreamRequest::RequestTrack via P2P
        // 2. Receive StreamResponse::Header
        // 3. Configure MediaCodec with format info
        // 4. Stream chunks -> buffer -> decoder -> AudioTrack
    }
    
    fun seek(position: Long) {
        // For large files: send StreamRequest::Seek
        // For pre-buffered: seek in buffer directly
    }
}
```

### 4. UI/UX Requirements

#### Main Screens
1. **Discovery Screen** (First Launch)
   - Auto-scan for servers
   - Show list of discovered desktops
   - Manual IP entry option
   - Remember last connected device

2. **Now Playing Screen**
   - Large album art (full-width, rounded corners)
   - Track title, artist, album
   - Progress bar with time (seekable)
   - Playback controls: Previous, Play/Pause, Next
   - Volume slider
   - Shuffle/Repeat toggles
   - Queue button (opens queue sheet)
   - Connection quality indicator (P2P/HTTP)
   - Bitrate/format display (FLAC 44.1kHz 16-bit, etc.)

3. **Library Screen** (Bottom Nav)
   - Tabs: Songs, Albums, Artists, Playlists
   - Search bar at top
   - Infinite scroll with pagination
   - Cover art thumbnails
   - Sort/filter options

4. **Queue Screen** (Bottom Sheet)
   - Current queue with drag-to-reorder
   - Clear queue option
   - Save as playlist

5. **Settings Screen**
   - Server connection settings
   - Audio quality preferences (auto/force FLAC/MP3)
   - Download cache size limit
   - Dark/light theme toggle
   - About/version info

#### Design Guidelines
- **Style**: Modern Material Design 3
- **Theme**: Dark theme primary, light theme optional
- **Colors**: Match desktop app (purple/blue accent colors)
- **Animations**: Smooth transitions, animated album art blur backgrounds
- **Typography**: Roboto/System font, clear hierarchy
- **Spacing**: 16dp standard padding, 8dp compact

### 5. Additional Features

#### Must Have
- [x] Background playback with notification controls
- [x] Lock screen media controls (MediaSession)
- [x] Headphone button controls
- [x] Bluetooth/car play integration
- [x] Network error handling with auto-reconnect
- [x] Offline queue (remember what was playing)
- [x] Album art caching

#### Nice to Have
- [ ] Download tracks for offline playback
- [ ] Playlist creation/editing
- [ ] Sleep timer
- [ ] Equalizer
- [ ] Chromecast support
- [ ] Android Auto integration
- [ ] Lyrics display (synced from desktop)

---

## Technology Stack Recommendations

### Primary Stack (Kotlin + Jetpack Compose)
```kotlin
// Core
- Language: Kotlin 2.0+
- UI: Jetpack Compose (Material 3)
- Architecture: MVVM + Clean Architecture

// Networking
- HTTP/REST: Retrofit + OkHttp
- WebSocket: OkHttp WebSocket
- P2P: libp2p-android or WebRTC native
- Serialization: kotlinx.serialization or Moshi

// Audio
- Decoder: MediaCodec (platform native)
- Playback: AudioTrack + ExoPlayer (for buffering)
- Background: MediaSessionService + MediaNotification

// Storage
- Database: Room (for cache, queue, settings)
- Preferences: DataStore

// Discovery
- mDNS: JmDNS or Android NSD API

// Dependencies
dependencies {
    // UI
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.activity:activity-compose")
    implementation("androidx.navigation:navigation-compose")
    
    // Network
    implementation("com.squareup.retrofit2:retrofit")
    implementation("com.squareup.okhttp3:okhttp")
    implementation("com.jakewharton.retrofit:retrofit2-kotlinx-serialization-converter")
    
    // Audio
    implementation("androidx.media3:media3-exoplayer")
    implementation("androidx.media3:media3-session")
    implementation("androidx.media3:media3-ui")
    
    // Discovery
    implementation("org.jmdns:jmdns:3.5.9")
    
    // Storage
    implementation("androidx.room:room-runtime")
    implementation("androidx.datastore:datastore-preferences")
    
    // Image loading
    implementation("io.coil-kt:coil-compose")
    
    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android")
}
```

### Alternative Stack (Flutter)
If cross-platform is desired:
- Framework: Flutter 3.x
- HTTP: `dio` package
- WebSocket: `web_socket_channel`
- Audio: `just_audio` + custom platform channel for MediaCodec
- P2P: Custom platform channel with native Kotlin/Swift code
- Discovery: `nsd` package for mDNS

---

## Implementation Phases

### Phase 1: Basic Connection (Week 1)
- [ ] Project setup with Jetpack Compose
- [ ] mDNS discovery implementation
- [ ] HTTP REST API client (Retrofit)
- [ ] WebSocket connection
- [ ] Library browsing UI (list tracks/albums)
- [ ] Basic playback via HTTP streaming

### Phase 2: P2P Streaming (Week 2)
- [ ] libp2p/WebRTC integration
- [ ] P2P connection establishment
- [ ] CBOR protocol implementation
- [ ] Streaming protocol handler
- [ ] MediaCodec integration
- [ ] Adaptive buffer implementation

### Phase 3: Polish & Features (Week 3)
- [ ] Now Playing UI with animations
- [ ] Queue management
- [ ] Background playback service
- [ ] MediaSession controls
- [ ] Cover art caching
- [ ] Error handling & reconnection
- [ ] Settings screen

### Phase 4: Testing & Optimization (Week 4)
- [ ] Network quality testing (WiFi/mobile data)
- [ ] Battery optimization
- [ ] Memory leak fixes
- [ ] Performance profiling
- [ ] Edge case handling
- [ ] UI polish

---

## Key Integration Points

### Desktop Server State (Reference)
```rust
// Located in src-tauri/src/lib.rs
pub struct AppState {
    pub player: Mutex<Option<AudioPlayer>>,
    pub db: Mutex<Option<DatabaseManager>>,
    pub p2p_manager: Arc<TokioRwLock<Option<P2PManager>>>,
    pub server_running: Arc<Mutex<bool>>,
}

// P2P Manager commands
pub enum P2PCommand {
    StreamToPeer { peer_id, track_path, start_byte },
    RequestStream { peer_id, track_path, start_byte },
    StopStream,
    ConnectPeer { multiaddr },
    GetPeers,
    Seek { byte_offset },
    Shutdown,
}

// Server events
pub enum ServerEvent {
    MediaSession { track_id, title, artist, album, duration, cover_url, is_playing, position },
    PlaybackState { is_playing, position, volume },
    WebRTCOffer { from_client_id, offer },
    WebRTCAnswer { to_client_id, answer },
}
```

### Mobile App Architecture
```
┌─────────────────────────────────────────┐
│         Mobile App (Android)            │
├─────────────────────────────────────────┤
│  UI Layer (Compose)                     │
│    - NowPlayingScreen                   │
│    - LibraryScreen                      │
│    - QueueScreen                        │
├─────────────────────────────────────────┤
│  ViewModel Layer                        │
│    - PlaybackViewModel                  │
│    - LibraryViewModel                   │
│    - ConnectionViewModel                │
├─────────────────────────────────────────┤
│  Repository Layer                       │
│    - MusicRepository (REST API)         │
│    - StreamRepository (P2P)             │
│    - DiscoveryRepository (mDNS)         │
├─────────────────────────────────────────┤
│  Service Layer                          │
│    - P2PAudioService (Background)       │
│    - WebSocketService (State Sync)      │
├─────────────────────────────────────────┤
│  Network Layer                          │
│    - RetrofitClient (HTTP)              │
│    - OkHttpWebSocket (WS)               │
│    - LibP2PClient (QUIC Stream)         │
└─────────────────────────────────────────┘
           ↓  mDNS Discovery
           ↓  HTTP/REST API (metadata)
           ↓  WebSocket (control)
           ↓  P2P QUIC (audio stream)
┌─────────────────────────────────────────┐
│      Desktop Server (Already Built)     │
│      - REST API on port 5443            │
│      - WebSocket /ws                    │
│      - P2P libp2p with QUIC             │
│      - Streaming protocol handler       │
└─────────────────────────────────────────┘
```

---

## Testing Requirements

### Manual Testing Checklist
- [ ] Discover server on same WiFi network
- [ ] Connect and browse library
- [ ] Play FLAC track with P2P streaming
- [ ] Seek to different positions
- [ ] Switch tracks (previous/next)
- [ ] Background playback works
- [ ] Notification controls functional
- [ ] Reconnect after WiFi disconnect
- [ ] Handle server restart gracefully
- [ ] Large file (>100MB) streaming works
- [ ] Battery usage acceptable

### Network Scenarios
- [ ] Excellent WiFi (low latency, high bandwidth)
- [ ] Poor WiFi (packet loss, high latency)
- [ ] WiFi → Mobile data handoff
- [ ] Airplane mode → reconnect
- [ ] Server goes offline → error UI
- [ ] Multiple clients connected simultaneously

---

## Performance Targets
- **Startup Time**: < 2 seconds to main screen
- **Discovery Time**: < 3 seconds to find server
- **P2P Connection**: < 2 seconds to establish
- **Playback Start**: < 500ms after tap (pre-buffered), < 2s (streaming)
- **Seek Latency**: < 200ms (pre-buffered), < 1s (re-stream)
- **Memory Usage**: < 150MB during playback
- **Battery Drain**: < 5% per hour of streaming
- **Network Usage**: Match file bitrate (no overhead >10%)

---

## Security Considerations
- **P2P Encryption**: Noise protocol (already implemented in desktop)
- **Local Network Only**: No internet exposure required
- **No Authentication**: Trust local network (add basic auth later if needed)
- **TLS**: Optional HTTPS for REST API (not implemented yet)

---

## Deliverables
1. **Android APK** - Release build signed for installation
2. **Source Code** - Clean, documented Kotlin codebase
3. **README.md** - Setup instructions, architecture overview
4. **User Guide** - How to connect, troubleshooting common issues
5. **Demo Video** - Show discovery, connection, and playback

---

## Success Criteria
✅ App discovers desktop server automatically on LAN  
✅ Connects via P2P and streams FLAC without quality loss  
✅ Playback is smooth with no stuttering or buffering pauses  
✅ Seeking works instantly for small files  
✅ Background playback continues when app is minimized  
✅ UI is responsive and matches modern Android design standards  
✅ Network errors are handled gracefully with reconnection  
✅ Battery usage is acceptable for typical listening sessions  

---

## Notes for Implementation

### P2P Library Options for Android
1. **WebRTC Native** - Best compatibility, Google maintains Android bindings
   - Use `org.webrtc:google-webrtc` dependency
   - Already has QUIC support
   - Easy integration with WebSocket signaling

2. **libp2p-android** - Direct port (if available)
   - May need custom JNI bindings
   - More complex but matches desktop exactly

3. **Hybrid Approach** (Recommended)
   - Use WebRTC for P2P connection establishment
   - Custom CBOR protocol over DataChannel
   - Fallback to HTTP streaming if P2P fails

### Critical Implementation Details
- **Chunk Size**: 64KB chunks (matches desktop CHUNK_SIZE)
- **Pre-buffer Threshold**: 20MB (matches desktop PREBUFFER_THRESHOLD)
- **Ring Buffer**: 3 seconds of audio data for large files
- **Timeout**: 5 minutes for large file transfers (matches desktop timeout)
- **Keep-Alive**: Ping every 30 seconds when idle

### Desktop Server Capabilities (Already Implemented)
The desktop server at `src-tauri/src/server/` and `src-tauri/src/p2p/` already supports:
- ✅ mDNS advertisement (stub - you may need to use proper mDNS library)
- ✅ HTTP REST endpoints for all metadata
- ✅ WebSocket for real-time state sync
- ✅ WebRTC signaling relay (offers/answers)
- ✅ P2P streaming protocol with CBOR codec
- ✅ Adaptive buffering strategy
- ✅ Cover art extraction and serving
- ✅ Original file byte passthrough (no re-encoding)

---

## Contact & Questions
If you need clarification on any technical details:
- Desktop server code: `src-tauri/src/server/` and `src-tauri/src/p2p/`
- Protocol definitions: `src-tauri/src/p2p/protocol.rs`
- Server routes: `src-tauri/src/server/routes.rs`
- WebSocket handler: `src-tauri/src/server/websocket.rs`

Good luck with the implementation! 🎵
