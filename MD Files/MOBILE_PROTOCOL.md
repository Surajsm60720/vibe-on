# VIBE-ON! Mobile Client Protocol Reference

This document describes the communication protocol used by the VIBE-ON! mobile app to connect to the desktop server.

## Overview

- **Transport**: HTTP REST API + WebSocket for real-time events
- **Default Port**: `5443`
- **Discovery**: mDNS (`_vibe-on._tcp`)
- **Serialization**: JSON with **camelCase** field names (Kotlinx.serialization)

---

## WebSocket Connection

### Endpoint
```
ws://{desktop-ip}:5443/control
```

### Connection Flow
1. Mobile connects to WebSocket endpoint
2. Mobile sends `GetState` message (or `Hello` with client name)
3. Desktop responds with `Welcome` message containing client ID
4. Desktop sends current `mediaSession` and `status` messages
5. Ongoing bidirectional communication for control and state updates

---

## Mobile → Desktop Messages (WebSocketMessage)

Discriminator field: `type` (Kotlinx.serialization default)

### Playback Controls
```kotlin
@SerialName("PlayPause") data object PlayPause
@SerialName("Play") data object Play
@SerialName("Pause") data object Pause
@SerialName("Stop") data object Stop
@SerialName("Next") data object Next
@SerialName("Previous") data object Previous
```

JSON:
```json
{ "type": "PlayPause" }
{ "type": "Play" }
{ "type": "Pause" }
```

### Seek
```kotlin
@SerialName("Seek")
data class Seek(@SerialName("position") val position: Double)
```

JSON:
```json
{
  "type": "Seek",
  "position": 45.5
}
```

### Set Volume
```kotlin
@SerialName("SetVolume")
data class SetVolume(@SerialName("volume") val volume: Float)
```

JSON:
```json
{
  "type": "SetVolume",
  "volume": 0.75
}
```

### Set Shuffle / Repeat
```kotlin
@SerialName("SetShuffle")
data class SetShuffle(@SerialName("enabled") val enabled: Boolean)

@SerialName("SetRepeat")
data class SetRepeat(@SerialName("mode") val mode: String)
```

JSON:
```json
{ "type": "SetShuffle", "enabled": true }
{ "type": "SetRepeat", "mode": "all" }
```

### Play Specific Track
```kotlin
@SerialName("PlayTrack")
data class PlayTrack(@SerialName("track_path") val trackPath: String)
```

JSON:
```json
{
  "type": "PlayTrack",
  "track_path": "/path/to/song.mp3"
}
```

### Queue Management
```kotlin
@SerialName("QueueTrack")
data class QueueTrack(@SerialName("track_path") val trackPath: String)

@SerialName("ClearQueue")
data object ClearQueue
```

### Get Current State
```kotlin
@SerialName("GetState")
data object GetState
```

---

## Desktop → Mobile Messages (ServerEvent)

Discriminator field: `type` (matches Kotlinx.serialization default)

### MediaSession (Track Info)
```kotlin
@SerialName("mediaSession")
data class MediaSession(
    @SerialName("trackId") val trackId: String? = null,
    @SerialName("title") val title: String? = null,
    @SerialName("artist") val artist: String? = null,
    @SerialName("album") val album: String? = null,
    @SerialName("duration") val duration: Double = 0.0,
    @SerialName("coverUrl") val coverUrl: String? = null,
    @SerialName("isPlaying") val isPlaying: Boolean = false,
    @SerialName("position") val position: Double = 0.0,
    @SerialName("timestamp") val timestamp: Long = 0L
)
```

JSON from desktop:
```json
{
  "type": "mediaSession",
  "trackId": "/path/to/song.mp3",
  "title": "Song Title",
  "artist": "Artist Name",
  "album": "Album Name",
  "duration": 234.5,
  "coverUrl": "/cover/%2Fpath%2Fto%2Fsong.mp3",
  "isPlaying": true,
  "position": 45.2,
  "timestamp": 1738339200000
}
```

**⚠️ Important Notes:**
- `trackId` is the **file path**, not a unique ID
- `trackId` is an **empty string `""`** when no track is loaded (NOT null)
- Handle empty string as "no track playing"
- `coverUrl` is relative - prepend server base URL: `http://{ip}:5443{coverUrl}`
- `timestamp` is epoch milliseconds when position was captured
- Calculate current position: `position + (System.currentTimeMillis() - timestamp) / 1000.0`

### Status
```kotlin
@SerialName("status")
data class Status(
    @SerialName("shuffle") val shuffle: Boolean = false,
    @SerialName("repeat") val repeat: String = "off",
    @SerialName("volume") val volume: Double = 1.0
)
```

JSON from desktop:
```json
{
  "type": "status",
  "volume": 0.75,
  "shuffle": false,
  "repeatMode": "off",
  "output": "desktop"
}
```

**Note:** Desktop sends `repeatMode`, mobile expects `repeat`. Add `@SerialName("repeatMode")` or handle both.

### Connected (Welcome)
```kotlin
@SerialName("Connected")
data class Connected(@SerialName("client_id") val clientId: String)
```

### Error
```kotlin
@SerialName("Error")
data class Error(@SerialName("message") val message: String)
```

---

## REST API Usage

Base URL: `http://{desktop-ip}:5443`

### VibeOnApi Interface

```kotlin
interface VibeOnApi {
    @GET("/health")
    suspend fun health(): Response<Unit>
    
    @GET("/api/info")
    suspend fun getInfo(): Response<ServerInfo>
    
    @GET("/api/playback")
    suspend fun getPlayback(): Response<PlaybackState>
    
    @GET("/api/library")
    suspend fun getLibrary(
        @Query("offset") offset: Int = 0,
        @Query("limit") limit: Int = 50
    ): Response<LibraryResponse>
    
    @GET("/api/albums")
    suspend fun getAlbums(): Response<List<Album>>
    
    @GET("/api/artists")
    suspend fun getArtists(): Response<List<Artist>>
    
    @GET("/api/lyrics/{track_path}")
    suspend fun getLyrics(
        @Path("track_path", encoded = true) trackPath: String
    ): Response<LyricsResponse>
    
    @GET("/cover/{track_path}")
    suspend fun getCover(
        @Path("track_path", encoded = true) trackPath: String
    ): Response<ResponseBody>
    
    @Streaming
    @GET("/stream")
    suspend fun stream(
        @Query("start") start: Long = 0
    ): Response<ResponseBody>
}
```

---

## Data Models

### Track
```kotlin
@Serializable
data class Track(
    @SerialName("id") val id: String = "",
    @SerialName("title") val title: String = "Unknown Track",
    @SerialName("artist") val artist: String = "Unknown Artist",
    @SerialName("album") val album: String = "Unknown Album",
    @SerialName("album_artist") val albumArtist: String? = null,
    @SerialName("track_number") val trackNumber: Int? = null,
    @SerialName("disc_number") val discNumber: Int? = null,
    @SerialName("year") val year: Int? = null,
    @SerialName("genre") val genre: String? = null,
    @SerialName("duration") val duration: Double = 0.0,
    @SerialName("path") val path: String = "",
    @SerialName("format") val format: String = "",
    @SerialName("sample_rate") val sampleRate: Int = 44100,
    @SerialName("bit_depth") val bitDepth: Int? = null,
    @SerialName("bitrate") val bitrate: Int? = null,
    @SerialName("channels") val channels: Int = 2,
    @SerialName("file_size") val fileSize: Long = 0
)
```

### PlaybackState (from REST API)
```kotlin
@Serializable
data class PlaybackState(
    @SerialName("is_playing") val isPlaying: Boolean = false,
    @SerialName("position") val position: Double = 0.0,
    @SerialName("duration") val duration: Double = 0.0,
    @SerialName("volume") val volume: Float = 1.0f,
    @SerialName("shuffle") val shuffle: Boolean = false,
    @SerialName("repeat") val repeat: RepeatMode = RepeatMode.OFF,
    @SerialName("current_track") val currentTrack: Track? = null
)

@Serializable
enum class RepeatMode {
    @SerialName("off") OFF,
    @SerialName("all") ALL,
    @SerialName("one") ONE
}
```

---

## Field Mapping: WebSocket vs REST

| Field | WebSocket (mediaSession) | REST (/api/playback) |
|-------|--------------------------|----------------------|
| Track ID | `trackId` (path) | `currentTrack.path` |
| Duration | `duration` | `durationSecs` |
| Position | `position` | `positionSecs` |
| Playing | `isPlaying` | `isPlaying` |
| Repeat | (in `status`: `repeatMode`) | `repeatMode` |

**⚠️ Handle both naming conventions or normalize in repository layer.**

---

## Timing & Keepalive

- **Desktop broadcasts** `mediaSession` every **2 seconds**
- **Mobile should send** `Ping` every **30 seconds** to prevent NAT timeout
- **Desktop responds** with `Pong` immediately
- **Connection timeout**: Reconnect if no messages for 60+ seconds

---

## Discovery

### mDNS
Service type: `_vibe-on._tcp`

```kotlin
class DiscoveryService {
    // Use Android NsdManager to discover _vibe-on._tcp services
    // Each discovered service provides IP and port (default 5443)
}
```

---

## Common Issues

### 1. Track ID is Empty String
Desktop sends `trackId: ""` when no track is playing. Don't check for null:
```kotlin
// ❌ Wrong
if (trackId != null) { ... }

// ✅ Correct
if (trackId.isNotEmpty()) { ... }
```

### 2. Cover URL is Relative
```kotlin
// ❌ Wrong
Glide.with(context).load(mediaSession.coverUrl)

// ✅ Correct
val fullUrl = "http://${serverIp}:5443${mediaSession.coverUrl}"
Glide.with(context).load(fullUrl)
```

### 3. Position Drift
Use timestamp to calculate accurate position:
```kotlin
val currentPosition = mediaSession.position + 
    (System.currentTimeMillis() - mediaSession.timestamp) / 1000.0
```

### 4. RepeatMode Field Name Mismatch
Desktop sends `repeatMode` in status, mobile might expect `repeat`:
```kotlin
@SerialName("repeatMode")  // Match desktop field name
val repeatMode: String = "off"
```
