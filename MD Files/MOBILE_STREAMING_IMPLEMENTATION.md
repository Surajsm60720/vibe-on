# Mobile Audio Streaming Implementation

## Overview

This document describes the implementation of mobile audio playback functionality, allowing the Android app to stream and play music from the PC server, with seamless handoff between desktop and mobile playback.

## Architecture

### PC (Server) Side

#### 1. **WebSocket Protocol Extensions** ([websocket.rs](vibe-on/src-tauri/src/server/websocket.rs))

Added new message types for mobile playback control:

**Client Messages:**
- `StartMobilePlayback` - Request to start streaming audio to mobile
- `StopMobilePlayback` - Request to stop mobile streaming and return to desktop
- `MobilePositionUpdate { position_secs: f64 }` - Sync playback position from mobile

**Server Messages:**
- `HandoffPrepare { sample: u64, url: String }` - Provides stream URL and position to mobile
- `StreamStopped` - Notifies that mobile streaming has stopped

#### 2. **Player Store** ([playerStore.ts](vibe-on/src/store/playerStore.ts))

Added:
- `audioOutput: 'desktop' | 'mobile'` - Current output destination
- `setAudioOutput(output)` - Change output and emit event to mobile clients

#### 3. **UI Components** ([PlayerBar.tsx](vibe-on/src/components/PlayerBar.tsx))

Added output selector button:
- 🔊 **Desktop** icon when playing on PC speakers
- 📱 **Mobile** icon when streaming to phone
- Toggle between outputs with single click

#### 4. **Event Listeners** ([App.tsx](vibe-on/src/App.tsx))

Added listener for `output-changed` events to handle:
- Pausing PC playback when mobile starts
- Resuming PC playback when mobile stops

### Android (Client) Side

#### 1. **WebSocketClient** ([WebSocketClient.kt](vibe-on-android/app/src/main/java/moe/memesta/vibeon/data/WebSocketClient.kt))

Added:
- `streamUrl: StateFlow<String?>` - Stream URL from server
- `isMobilePlayback: StateFlow<Boolean>` - Whether mobile is actively playing
- `sendStartMobilePlayback()` - Request stream URL from server
- `sendStopMobilePlayback()` - Stop streaming and return to desktop
- `sendMobilePositionUpdate(positionSecs)` - Sync position to server
- Handler for `handoffPrepare` and `streamStopped` messages

#### 2. **PlaybackViewModel** ([PlaybackViewModel.kt](vibe-on-android/app/src/main/java/moe/memesta/vibeon/ui/PlaybackViewModel.kt))

Complete rewrite to handle streaming:
- Integrates with `WebSocketClient` to receive stream URLs
- Controls `ExoPlayer` to play HTTP streams
- Syncs playback position with PC via WebSocket
- Provides `requestMobilePlayback()` and `stopMobilePlayback()` methods

#### 3. **NowPlayingScreen** ([NowPlayingScreen.kt](vibe-on-android/app/src/main/java/moe/memesta/vibeon/ui/NowPlayingScreen.kt))

Added:
- `isMobilePlayback` parameter
- Output location button showing "Playing on Desktop" or "Playing on Mobile"
- Toggle handler `onTogglePlaybackLocation()`

#### 4. **MainActivity** ([MainActivity.kt](vibe-on-android/app/src/main/java/moe/memesta/vibeon/MainActivity.kt))

Wired up:
- `PlaybackViewModel` with `WebSocketClient` and `MediaController`
- Output toggle button to call `requestMobilePlayback()` / `stopMobilePlayback()`
- State flow for `isMobilePlayback` from WebSocket

## User Flow

### Starting Mobile Playback

1. **User clicks output button** (Desktop → Mobile)
   - PC: `setAudioOutput('mobile')` called
   - PC: Emits `output-changed` event
   - Mobile: Receives output change via WebSocket
   
2. **Mobile requests stream**
   - Mobile: Sends `startMobilePlayback` message
   - PC: Pauses Rodio player
   - PC: Constructs stream URL: `http://<ip>:5000/stream/<encoded_path>`
   - PC: Sends `handoffPrepare` with URL and current position
   
3. **Mobile starts playback**
   - Mobile: Receives stream URL
   - Mobile: Creates `MediaItem` from URL
   - Mobile: Initializes ExoPlayer with media item
   - Mobile: Starts playback at synced position
   - Mobile: UI shows "Playing on Mobile"

### Stopping Mobile Playback

1. **User clicks output button** (Mobile → Desktop)
   - Mobile: Sends `stopMobilePlayback` message
   - Mobile: Stops ExoPlayer
   - PC: Receives message, resumes Rodio player
   - PC: Emits `output-changed` event with output='desktop'
   - Mobile: Receives `streamStopped` message
   - Mobile: UI shows "Playing on Desktop"

## HTTP Streaming

### Endpoint
```
GET /stream/{encoded_path}
```

### Example
```
http://192.168.1.10:5000/stream/%2Fhome%2Fuser%2FMusic%2Fsong.mp3
```

### Headers
- `Content-Type: audio/mpeg` (or appropriate MIME type)
- `Accept-Ranges: bytes` (supports seeking)

### Implementation
The existing HTTP server in [routes.rs](vibe-on/src-tauri/src/server/routes.rs) already handles streaming:
```rust
pub async fn stream_audio_file(Path(encoded_path): Path<String>) -> Response {
    let track_path = urlencoding::decode(&encoded_path).to_string();
    let data = tokio::fs::read(&track_path).await?;
    Response::builder()
        .header(CONTENT_TYPE, "audio/mpeg")
        .header(ACCEPT_RANGES, "bytes")
        .body(Body::from(data))
}
```

## State Synchronization

### PC → Mobile
- **Track metadata**: Broadcast every 2 seconds via `mediaSession` message
- **Playback state**: `isPlaying`, `position`, `volume`
- **Queue updates**: When queue changes

### Mobile → PC
- **Position updates**: Sent periodically during mobile playback
- **Control commands**: Play, pause, seek, volume, next, prev
- **Playback location**: Start/stop mobile streaming

## Key Files Modified

### PC (TypeScript/Rust)
1. `src/store/playerStore.ts` - Added `audioOutput` state and `setAudioOutput` action
2. `src/components/PlayerBar.tsx` - Added output selector UI
3. `src/App.tsx` - Added `output-changed` event listener
4. `src-tauri/src/server/websocket.rs` - Added mobile playback messages and handlers

### Android (Kotlin)
1. `app/src/main/java/moe/memesta/vibeon/data/WebSocketClient.kt` - Stream URL handling
2. `app/src/main/java/moe/memesta/vibeon/ui/PlaybackViewModel.kt` - ExoPlayer integration
3. `app/src/main/java/moe/memesta/vibeon/ui/NowPlayingScreen.kt` - Output toggle UI
4. `app/src/main/java/moe/memesta/vibeon/MainActivity.kt` - Wired up ViewModel and UI

## Testing

### Manual Testing Steps

1. **Start PC app** with music library
2. **Connect Android app** via device discovery
3. **Play a track** on PC (should hear on PC speakers)
4. **Click mobile icon** in PC player bar
5. **Verify** audio switches to phone
6. **Check** that PC playback is paused
7. **Seek** on mobile app
8. **Verify** position syncs between devices
9. **Click desktop icon** to switch back
10. **Verify** audio returns to PC speakers

### Edge Cases to Test

- [ ] Network disconnection during playback
- [ ] Switching tracks while on mobile playback
- [ ] Volume changes during mobile playback
- [ ] Queue modifications during mobile playback
- [ ] Multiple mobile clients (should only allow one to stream)

## Future Enhancements

### 1. **P2P Streaming** (Already Scaffolded)
- Use the existing libp2p infrastructure in `vibe-on-core`
- Lower latency direct peer-to-peer streaming
- No need to route through HTTP server

### 2. **Buffering Strategy**
- Pre-buffer next track in queue
- Cache recent tracks for instant replay
- Adaptive bitrate based on network conditions

### 3. **Background Playback**
- Android `ForegroundService` for persistent playback
- Media notification with controls
- Handle audio focus and interruptions

### 4. **Multi-Room Audio**
- Support multiple mobile devices streaming simultaneously
- Synchronized playback across devices
- Group volume control

### 5. **Codec Transcoding**
- Transcode FLAC → MP3 for mobile data saving
- Adjustable bitrate based on WiFi/cellular
- Server-side transcoding pipeline

## Performance Considerations

### Network
- **Bandwidth**: ~320 kbps for high-quality MP3, ~1-1.4 Mbps for FLAC
- **Latency**: HTTP has ~100-500ms buffering overhead
- **WiFi required**: Cellular would consume significant data

### Battery
- ExoPlayer is efficient but constant streaming drains battery
- Consider wake locks and battery optimization exemptions

### Memory
- ExoPlayer buffers ~15-30 seconds ahead by default
- Monitor memory usage for long listening sessions

## Troubleshooting

### "No audio on mobile"
- Check that stream URL is reachable (ping PC IP from mobile)
- Verify HTTP server is running on port 5000
- Check firewall rules on PC

### "Playback stutters"
- Weak WiFi signal - move closer to router
- PC CPU overload - check system resources
- Network congestion - check other devices on network

### "Can't switch back to desktop"
- Check WebSocket connection (reconnect if dropped)
- Restart both PC and mobile apps
- Check logs for `stopMobilePlayback` message

## Logs

### PC Logs
```bash
# View WebSocket messages
grep "WebSocket" output.txt

# View streaming requests
grep "stream" output.txt
```

### Android Logs
```bash
# Filter app logs
adb logcat | grep "Vibeon\|WebSocket\|PlaybackViewModel"
```

## Conclusion

The mobile streaming implementation provides seamless audio playback handoff between PC and mobile devices using HTTP streaming and WebSocket control. The architecture is designed for future expansion to P2P streaming and multi-room audio.
