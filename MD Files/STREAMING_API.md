# VIBE-ON! Streaming API Documentation

## Base URLs
- **HTTP REST API:** `http://{PC_IP}:5000`
- **WebSocket Control:** `ws://{PC_IP}:5443/control`

## HTTP REST API Endpoints

### Server Information

#### `GET /api/info`
Get server information and library size.

**Response:**
```json
{
  "name": "My PC",
  "version": "0.1.0",
  "platform": "linux",
  "librarySize": 1234,
  "port": 5000,
  "localIp": "192.168.1.100"
}
```

**Status Codes:**
- `200 OK` - Success
- `503 Service Unavailable` - Server not ready

---

### Library Browsing

#### `GET /api/library`
Get library tracks with pagination.

**Query Parameters:**
- `offset` (optional, default: 0) - Starting position
- `limit` (optional, default: 50) - Number of tracks to return

**Example:** `GET /api/library?offset=0&limit=50`

**Response:**
```json
{
  "tracks": [
    {
      "path": "/home/user/Music/Artist/Album/Song.mp3",
      "title": "Song Title",
      "artist": "Artist Name",
      "album": "Album Name",
      "durationSecs": 180.5,
      "discNumber": 1,
      "trackNumber": 3,
      "coverUrl": "/cover/%2Fhome%2Fuser%2FMusic%2FArtist%2FAlbum%2FSong.mp3"
    }
  ],
  "total": 1234
}
```

**Status Codes:**
- `200 OK` - Success
- `400 Bad Request` - Invalid parameters
- `503 Service Unavailable` - Database error

---

#### `GET /api/library/search`
Full-text search in library.

**Query Parameters:**
- `q` (required) - Search query
- `offset` (optional, default: 0) - Starting position
- `limit` (optional, default: 50) - Number of results

**Example:** `GET /api/library/search?q=artist%20name&limit=20`

**Response:**
```json
{
  "tracks": [
    {
      "path": "/path/to/song.mp3",
      "title": "Song Title",
      "artist": "Artist Name",
      "album": "Album Name",
      "durationSecs": 180.5,
      "coverUrl": "/cover/..."
    }
  ],
  "albums": [
    {
      "name": "Album Name",
      "artist": "Artist Name",
      "coverUrl": "/cover/...",
      "trackCount": 12
    }
  ],
  "artists": [
    {
      "name": "Artist Name",
      "albumCount": 5,
      "trackCount": 45
    }
  ]
}
```

**Status Codes:**
- `200 OK` - Success
- `400 Bad Request` - Missing query parameter
- `503 Service Unavailable` - Database error

---

### Albums & Artists

#### `GET /api/albums`
Get all albums with pagination.

**Query Parameters:**
- `offset` (optional, default: 0)
- `limit` (optional, default: 50)

**Response:**
```json
{
  "albums": [
    {
      "name": "Album Name",
      "artist": "Artist Name",
      "coverUrl": "/cover/...",
      "trackCount": 12
    }
  ],
  "total": 234
}
```

---

#### `GET /api/albums/:name/:artist`
Get album details with all tracks.

**Parameters:**
- `name` - Album name (URL encoded)
- `artist` - Artist name (URL encoded)

**Example:** `GET /api/albums/Dark%20Side%2FPink%20Floyd`

**Response:**
```json
{
  "album": {
    "name": "Album Name",
    "artist": "Artist Name",
    "coverUrl": "/cover/...",
    "trackCount": 12
  },
  "tracks": [
    {
      "path": "/path/to/track.mp3",
      "title": "Track 1",
      "artist": "Artist Name",
      "album": "Album Name",
      "durationSecs": 180.5,
      "trackNumber": 1,
      "coverUrl": "/cover/..."
    }
  ]
}
```

---

#### `GET /api/artists`
Get all artists with pagination.

**Query Parameters:**
- `offset` (optional, default: 0)
- `limit` (optional, default: 50)

**Response:**
```json
{
  "artists": [
    {
      "name": "Artist Name",
      "albumCount": 5,
      "trackCount": 45
    }
  ],
  "total": 1500
}
```

---

#### `GET /api/artists/:name`
Get artist details with albums and tracks.

**Parameters:**
- `name` - Artist name (URL encoded)

**Response:**
```json
{
  "artist": {
    "name": "Artist Name",
    "albumCount": 5,
    "trackCount": 45
  },
  "albums": [
    {
      "name": "Album Name",
      "artist": "Artist Name",
      "coverUrl": "/cover/...",
      "trackCount": 12
    }
  ],
  "tracks": [
    {
      "path": "/path/to/track.mp3",
      "title": "Track Name",
      "artist": "Artist Name",
      "album": "Album Name",
      "durationSecs": 180.5,
      "coverUrl": "/cover/..."
    }
  ]
}
```

---

### Playback State

#### `GET /api/playback`
Get current playback state.

**Response:**
```json
{
  "isPlaying": true,
  "currentTrack": {
    "path": "/path/to/song.mp3",
    "title": "Song Title",
    "artist": "Artist Name",
    "album": "Album Name",
    "durationSecs": 180.5,
    "coverUrl": "/cover/..."
  },
  "positionSecs": 45.3,
  "durationSecs": 180.5,
  "volume": 0.8,
  "shuffle": false,
  "repeatMode": "off",
  "queue": [
    {
      "path": "/path/to/next.mp3",
      "title": "Next Track",
      "artist": "Artist",
      "album": "Album",
      "durationSecs": 200.0,
      "coverUrl": "/cover/..."
    }
  ]
}
```

**Status Codes:**
- `200 OK` - Success
- `503 Service Unavailable` - Player not initialized

---

### Lyrics

#### `GET /api/lyrics/*path`
Get lyrics for a track.

**Parameters:**
- `path` - Full file path (URL encoded)

**Example:** `GET /api/lyrics/home/user/Music/song.mp3`

**Response:**
```json
{
  "trackPath": "/path/to/song.mp3",
  "hasSynced": true,
  "syncedLyrics": "[00:10.50]Line 1...",
  "plainLyrics": "Line 1\nLine 2\n...",
  "instrumental": false
}
```

**Status Codes:**
- `200 OK` - Success
- `404 Not Found` - Track not found
- `503 Service Unavailable` - Lyrics provider unavailable

---

### Cover Art

#### `GET /cover/*path`
Get cover art image for a track.

**Parameters:**
- `path` - Full file path (URL encoded)

**Example:** `GET /cover/home/user/Music/song.mp3`

**Response:**
- Binary image data (PNG or JPEG)
- `Content-Type: image/jpeg` or `image/png`

**Status Codes:**
- `200 OK` - Success with image data
- `404 Not Found` - No cover found
- `503 Service Unavailable` - File access error

---

### Audio Streaming

#### `GET /stream/:path`
Stream audio file for playback.

**Parameters:**
- `path` - Full file path (URL encoded)

**Example:** `GET /stream/home/user/Music/song.mp3`

**Response Headers:**
- `Content-Type: audio/mpeg` (or appropriate MIME type)
- `Content-Length: {size_in_bytes}`
- `Accept-Ranges: bytes`
- `Cache-Control: public, max-age=604800`

**Response:**
- Binary audio file data

**Supported Formats:**
- `audio/mpeg` (.mp3)
- `audio/flac` (.flac)
- `audio/wav` (.wav)
- `audio/aac` (.m4a, .aac)
- `audio/ogg` (.ogg)
- `audio/opus` (.opus)

**Status Codes:**
- `200 OK` - Success with audio data
- `400 Bad Request` - Invalid path encoding
- `404 Not Found` - File not found
- `503 Service Unavailable` - File access error

**HTTP Range Requests:**
The endpoint supports HTTP Range requests for seeking:
```
GET /stream/path HTTP/1.1
Range: bytes=1000-2000
```

Response will be `206 Partial Content` with the requested byte range.

---

#### `GET /stream?start={sample}` (Legacy)
Stream current playing track with start position.

**Query Parameters:**
- `start` (optional, default: 0) - Starting sample position

**Response:**
- Binary audio data
- Headers: `Content-Type`, `X-Audio-Format`, `X-Audio-Start-Sample`

---

## WebSocket Messages

### Client → Server Messages

All messages use JSON format with a required `type` field.

#### Connection

**Hello / Identify**
```json
{
  "type": "hello",
  "client_name": "My Phone"
}
```

#### Playback Control

**Play**
```json
{
  "type": "play"
}
```

**Pause**
```json
{
  "type": "pause"
}
```

**Resume**
```json
{
  "type": "resume"
}
```

**Stop**
```json
{
  "type": "stop"
}
```

**Next Track**
```json
{
  "type": "next"
}
```

**Previous Track**
```json
{
  "type": "previous"
}
```

**Seek**
```json
{
  "type": "seek",
  "position_secs": 45.5
}
```

**Set Volume** (0.0 - 1.0)
```json
{
  "type": "setVolume",
  "volume": 0.75
}
```

#### Queue Control

**Play Specific Track**
```json
{
  "type": "playTrack",
  "path": "/full/path/to/track.mp3"
}
```

**Play Album**
```json
{
  "type": "playAlbum",
  "album": "Album Name",
  "artist": "Artist Name"
}
```

**Play Artist**
```json
{
  "type": "playArtist",
  "artist": "Artist Name"
}
```

**Add to Queue**
```json
{
  "type": "addToQueue",
  "path": "/full/path/to/track.mp3"
}
```

**Set Queue**
```json
{
  "type": "setQueue",
  "paths": [
    "/path/1.mp3",
    "/path/2.mp3"
  ]
}
```

**Toggle Shuffle**
```json
{
  "type": "toggleShuffle"
}
```

**Cycle Repeat Mode** (off → all → one → off)
```json
{
  "type": "cycleRepeat"
}
```

#### Status

**Get Status**
```json
{
  "type": "getStatus"
}
```

**Ping** (keepalive)
```json
{
  "type": "ping"
}
```

---

### Server → Client Messages

#### Connection

**Connected / Welcome**
```json
{
  "type": "connected",
  "clientId": "unique-id"
}
```

**Media Session** (broadcast every 2 seconds)
```json
{
  "type": "mediaSession",
  "trackId": "path/to/track.mp3",
  "title": "Track Title",
  "artist": "Artist Name",
  "album": "Album Name",
  "duration": 180.5,
  "coverUrl": "/cover/...",
  "isPlaying": true,
  "position": 45.3,
  "timestamp": 1673000000000
}
```

**Status Update**
```json
{
  "type": "status",
  "volume": 0.8,
  "shuffle": false,
  "repeatMode": "off",
  "output": "speakers"
}
```

**Queue Update**
```json
{
  "type": "queueUpdate",
  "queue": [
    {
      "path": "/path/to/track1.mp3",
      "title": "Track 1",
      "artist": "Artist",
      "album": "Album",
      "durationSecs": 180.5,
      "coverUrl": "/cover/..."
    }
  ],
  "currentIndex": 0
}
```

**Lyrics**
```json
{
  "type": "lyrics",
  "trackPath": "/path/to/track.mp3",
  "hasSynced": true,
  "syncedLyrics": "[00:10.50]...",
  "plainLyrics": "...",
  "instrumental": false
}
```

**Error**
```json
{
  "type": "error",
  "message": "Error description"
}
```

**Pong** (response to ping)
```json
{
  "type": "pong"
}
```

**Handoff Prepare** (for streaming to mobile)
```json
{
  "type": "handoffPrepare",
  "sample": 44100,
  "url": "http://192.168.1.100:5000/stream/encoded_path"
}
```

**Handoff Commit** (start streaming on mobile)
```json
{
  "type": "handoffCommit"
}
```

**Stream Stopped** (returned to desktop)
```json
{
  "type": "streamStopped"
}
```

---

## Error Handling

All errors follow this pattern:

**HTTP Errors:**
- `400 Bad Request` - Invalid parameters
- `404 Not Found` - Resource not found
- `500 Internal Server Error` - Server error
- `503 Service Unavailable` - Service not ready

**WebSocket Errors:**
```json
{
  "type": "error",
  "message": "Descriptive error message"
}
```

---

## Rate Limiting

- No rate limiting on local LAN
- Recommended: Max 100 library requests/minute per client
- Streaming: Unlimited (constrained by network bandwidth)

---

## CORS

CORS is enabled for all origins (`*`), suitable for local LAN use only.

---

## Example Flow: Playing a Song

1. **Discover Server**
   ```
   mDNS query for _vibe-on._tcp
   → Get PC IP and port
   ```

2. **Connect WebSocket**
   ```
   ws://PC_IP:5443/control
   → Send: {"type": "hello", "client_name": "Android"}
   ← Receive: {"type": "connected", "clientId": "..."}
   ```

3. **Browse Library**
   ```
   GET http://PC_IP:5000/api/library?limit=50
   ← Receive: {tracks: [...], total: 1234}
   ```

4. **Play Track**
   ```
   Send: {"type": "playTrack", "path": "/full/path/to/song.mp3"}
   ← Receive: {"type": "mediaSession", ...}
   
   GET http://PC_IP:5000/stream/full%2Fpath%2Fto%2Fsong.mp3
   ← Receive: audio data
   ```

5. **Get Updates**
   ```
   Listen to WebSocket for:
   - mediaSession (track info, position)
   - status (volume, shuffle, repeat)
   - queueUpdate (queue changes)
   ```
