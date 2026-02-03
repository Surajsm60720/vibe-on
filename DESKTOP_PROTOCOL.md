# VIBE-ON! Desktop Server Protocol Reference

This document describes the communication protocol used by the VIBE-ON! desktop server for mobile connectivity.

## Overview

- **Transport**: HTTP REST API + WebSocket for real-time events
- **Default Port**: `5443`
- **Discovery**: mDNS (`_vibe-on._tcp`)
- **Serialization**: JSON with **camelCase** field names

---

## WebSocket Connection

### Endpoint
```
ws://{desktop-ip}:5443/control
```

### Connection Flow
1. Mobile connects to WebSocket endpoint
2. Mobile sends `Hello` message with client name
3. Desktop responds with `Welcome` message containing client ID
4. Desktop sends current `mediaSession` and `status` messages
5. Ongoing bidirectional communication for control and state updates

---

## Client → Server Messages

All messages use `type` as the discriminator field. Field names are **camelCase**.

### Hello (Handshake)
```json
{
  "type": "hello",
  "clientName": "iPhone 15 Pro"
}
```

### Playback Controls
```json
{ "type": "play" }
{ "type": "pause" }
{ "type": "resume" }
{ "type": "stop" }
{ "type": "next" }
{ "type": "previous" }
```

### Seek
```json
{
  "type": "seek",
  "positionSecs": 45.5
}
```

### Set Volume
```json
{
  "type": "setVolume",
  "volume": 0.75
}
```

### Toggle Shuffle / Cycle Repeat
```json
{ "type": "toggleShuffle" }
{ "type": "cycleRepeat" }
```

### Play Specific Track
```json
{
  "type": "playTrack",
  "path": "/path/to/song.mp3"
}
```

### Play Album / Artist
```json
{
  "type": "playAlbum",
  "album": "Album Name",
  "artist": "Artist Name"
}
```
```json
{
  "type": "playArtist",
  "artist": "Artist Name"
}
```

### Queue Management
```json
{
  "type": "addToQueue",
  "path": "/path/to/song.mp3"
}
```
```json
{
  "type": "setQueue",
  "paths": ["/path/to/song1.mp3", "/path/to/song2.mp3"]
}
```

### Get Current Status
```json
{ "type": "getStatus" }
```

### Get Lyrics
```json
{ "type": "getLyrics" }
```

### Ping (Keepalive)
```json
{ "type": "ping" }
```

### Audio Streaming
```json
{ "type": "requestStreamToMobile" }
{ "type": "stopStreamToMobile" }
{ "type": "handoffReady" }
```

### Network Stats
```json
{
  "type": "networkStats",
  "bufferMs": 500,
  "throughputKbps": 1200
}
```

---

## Server → Client Messages

All messages use `type` as the discriminator field. Field names are **camelCase**.

### Welcome
```json
{
  "type": "welcome",
  "clientId": "uuid-string",
  "serverName": "My Desktop",
  "version": "1.0.0"
}
```

### MediaSession (Track Info)
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

**Important Notes:**
- `trackId` is the file path, not a unique ID
- `trackId` is an **empty string `""`** when no track is loaded (not null)
- `coverUrl` is a relative URL path (prepend server base URL)
- `timestamp` is epoch milliseconds when the position was captured
- To calculate current position: `currentPosition = position + (Date.now() - timestamp) / 1000`

### Status
```json
{
  "type": "status",
  "volume": 0.75,
  "shuffle": false,
  "repeatMode": "off",
  "output": "desktop"
}
```

**RepeatMode values:** `"off"`, `"all"`, `"one"`

### Position Update
```json
{
  "type": "positionUpdate",
  "position": 46.5,
  "timestamp": 1738339201000
}
```

### Queue Update
```json
{
  "type": "queueUpdate",
  "tracks": [
    {
      "path": "/path/to/song.mp3",
      "title": "Song Title",
      "artist": "Artist Name",
      "album": "Album Name",
      "durationSecs": 234.5,
      "coverUrl": "/cover/..."
    }
  ]
}
```

### Lyrics
```json
{
  "type": "lyrics",
  "trackPath": "/path/to/song.mp3",
  "hasSynced": true,
  "syncedLyrics": "[00:05.00]First line\n[00:10.00]Second line",
  "plainLyrics": "First line\nSecond line",
  "instrumental": false
}
```

### Audio Handoff
```json
{
  "type": "handoffPrepare",
  "sample": 12345678,
  "url": "http://..."
}
```
```json
{ "type": "handoffCommit" }
{ "type": "streamStopped" }
```

### Error
```json
{
  "type": "error",
  "message": "Error description"
}
```

### Pong
```json
{ "type": "pong" }
```

---

## REST API Endpoints

Base URL: `http://{desktop-ip}:5443`

### Health Check
```
GET /health
Response: { "status": "ok" }
```

### Server Info
```
GET /api/info
Response: {
  "name": "My Desktop",
  "version": "1.0.0",
  "platform": "linux",
  "librarySize": 1234,
  "port": 5443,
  "localIp": "192.168.1.100"
}
```

### Playback State
```
GET /api/playback
Response: {
  "isPlaying": true,
  "currentTrack": {
    "path": "/path/to/song.mp3",
    "title": "Song Title",
    "artist": "Artist Name",
    "album": "Album Name",
    "durationSecs": 234.5,
    "discNumber": 1,
    "trackNumber": 5,
    "coverUrl": "/cover/..."
  },
  "positionSecs": 45.2,
  "durationSecs": 234.5,
  "volume": 0.75,
  "shuffle": false,
  "repeatMode": "off",
  "queue": []
}
```

### Library
```
GET /api/library?offset=0&limit=50
Response: {
  "tracks": [...],
  "total": 1234
}
```

### Search
```
GET /api/library/search?q=query&offset=0&limit=20
Response: {
  "tracks": [...],
  "albums": [...],
  "artists": [...]
}
```

### Albums
```
GET /api/albums
Response: {
  "albums": [
    {
      "name": "Album Name",
      "artist": "Artist Name",
      "coverUrl": "/cover/...",
      "trackCount": 12
    }
  ],
  "total": 50
}

GET /api/albums/{name}/{artist}
Response: {
  "album": {...},
  "tracks": [...]
}
```

### Artists
```
GET /api/artists
Response: {
  "artists": [
    {
      "name": "Artist Name",
      "albumCount": 5,
      "trackCount": 50
    }
  ],
  "total": 20
}

GET /api/artists/{name}
Response: {
  "artist": {...},
  "albums": [...],
  "tracks": [...]
}
```

### Lyrics
```
GET /api/lyrics/{url-encoded-path}
Response: {
  "trackPath": "/path/to/song.mp3",
  "hasSynced": true,
  "syncedLyrics": "...",
  "plainLyrics": "...",
  "instrumental": false
}
```

### Cover Art
```
GET /cover/{url-encoded-path}
Response: Binary image data (JPEG/PNG)
```

### Audio Stream
```
GET /stream?start=0
Response: Raw audio stream (PCM or encoded)
```

---

## Tauri Events (Internal)

Events emitted by the server for frontend integration:

- `mobile_client_connected` - Payload: `{ clientId, clientName }`
- `mobile_client_disconnected` - Payload: `{ clientId }`

---

## Timing & Keepalive

- **Server broadcasts** `mediaSession` every **2 seconds** to connected clients
- **Client should send** `ping` every **30 seconds** to prevent NAT timeout
- **Server responds** with `pong` immediately
- **Connection timeout**: If no messages for 60+ seconds, connection may be dropped

---

## Field Naming Convention

| Internal (Rust) | Serialized (JSON) |
|-----------------|-------------------|
| `track_id` | `trackId` |
| `is_playing` | `isPlaying` |
| `cover_url` | `coverUrl` |
| `repeat_mode` | `repeatMode` |
| `position_secs` | `positionSecs` |
| `duration_secs` | `durationSecs` |

All JSON uses **camelCase** due to `#[serde(rename_all = "camelCase")]`.
