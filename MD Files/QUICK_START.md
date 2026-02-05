# Quick Start: Local Music Streaming

## 📋 Prerequisites
- PC (Windows/Mac/Linux) with VIBE-ON! desktop app
- Android phone with VIBE-ON! mobile app
- Same WiFi network
- At least 50 Mbps bandwidth recommended

## 🚀 Desktop Setup (PC)

### 1. Build and Run
```bash
cd vibe-on
cargo tauri dev
```

### 2. Configure Library
```
Settings → Add Music Folder → Select your music directory
Wait for library to index...
```

### 3. Start Mobile Server
```
Menu → Start Mobile Server
✅ Server running on port 5000 (HTTP) and 5443 (WebSocket)
```

### 4. Find Your PC IP
**Windows:**
```cmd
ipconfig
# Look for IPv4 Address (e.g., 192.168.1.100)
```

**Mac/Linux:**
```bash
ifconfig
# Look for inet address (e.g., 192.168.1.100)
```

## 📱 Mobile Setup (Android)

### 1. Build and Install
```bash
cd vibe-on-android
./gradlew assembleDebug
# Or use Android Studio to build
```

### 2. Grant Permissions
- Network access (required)
- Bluetooth (optional, for headphones)

### 3. Open App
- App automatically discovers your PC
- You should see device name in the list

## 🎵 Using the App

### Discovery & Connection
1. **Desktop**: Server shows as available via mDNS
2. **Mobile**: "Discovery" screen shows "My PC" or your device name
3. **Tap**: Select your desktop device
4. **Wait**: Connects via WebSocket (< 2 seconds)

### Browsing Library
1. **All Songs**: Shows paginated list (50 songs per page)
2. **Search**: Type artist name or song title
3. **Albums**: Browse by album name
4. **Artists**: Browse by artist name
5. **Scroll**: Automatically loads more when near end

### Playing Music
1. **Tap Song**: Starts playback on both desktop and mobile
2. **Desktop**: Shows now playing with cover art
3. **Mobile**: Streams audio from desktop PC
4. **Controls**: Play, pause, skip, seek, volume

### Queue Management
1. **Queue** tab: Shows all queued songs
2. **Add**: Tap "+" to add songs to queue
3. **Reorder**: Drag songs to reorder (if supported)
4. **Auto-sync**: Desktop queue changes appear on mobile in real-time

## 🔧 Troubleshooting

### Mobile Can't Discover Desktop
```
❌ Problem: No devices showing in Discovery
✅ Solution:
  1. Both on same WiFi network? YES → Continue
  2. Desktop server running? YES → Continue
  3. Try: Restart WiFi on phone
  4. Try: Restart desktop app
  5. Check: Firewall not blocking port 5443
```

### Connected But No Music Plays
```
❌ Problem: Can't play songs
✅ Solution:
  1. Check: Desktop has music indexed
     Desktop → Menu → Rescan Music Library
  2. Check: File paths are valid
  3. Try: Refresh mobile (swipe down)
  4. Try: Reconnect (tap device again)
```

### Playback Stuttering/Slow
```
❌ Problem: Audio cuts or lags
✅ Solution:
  1. Check: WiFi signal strength (should be > -50 dBm)
  2. Try: Move closer to router
  3. Try: Close other apps using network
  4. Check: Desktop not CPU overloaded
  5. Note: Lower quality WiFi = stream will struggle
```

### Can't Connect to Desktop
```
❌ Problem: Connection fails/times out
✅ Solution:
  1. Verify IP address correct in error message
  2. Check: Desktop firewall allows port 5443
     Windows: Settings → Firewall → Allow app through
     Mac: System Preferences → Security → Firewall
     Linux: sudo ufw allow 5443
  3. Try: Disable WiFi → Re-enable → Try again
  4. Try: Restart desktop app
```

## 📊 Performance Tips

### Large Library (10k+ songs)
- Use **Search** instead of scrolling
- Increase pagination limit in settings if available
- Library indexes once, then cached

### Slow WiFi
- Keep phone close to router
- Close bandwidth-heavy apps on other devices
- Use 5GHz WiFi if available (faster than 2.4GHz)
- Original quality requires good signal; app will buffer if needed

### Battery Drain on Mobile
- Screen stays on during playback → normal
- Close app when not in use
- Lower screen brightness if extended use

## 📝 Common Tasks

### Change PC for Playback
1. **Mobile**: "Settings" → "Select Device"
2. **Or**: Tap different device in Discovery

### Rescan Music Library
1. **Desktop**: Menu → Settings → Rescan Music Library
2. **Wait**: (can take minutes for large libraries)
3. **Mobile**: Refresh to see new songs

### Clear App Cache
```bash
# Android Studio Device Manager
# Or adb commands
adb shell pm clear moe.memesta.vibeon
```

### View Server Logs (Desktop)
```bash
# Check console output for server logs
# Look for [Server] messages
```

## 📱 Keyboard Shortcuts

### Mobile
- **Volume up/down**: System volume buttons (controls playback volume)
- **Back button**: Returns to previous screen
- **Search icon**: Opens search screen

### Desktop
- **Space**: Play/Pause (when focused on player)
- **→**: Next track
- **←**: Previous track
- **↑/↓**: Adjust volume

## 🔐 Security Notes

- **Local Network Only**: No cloud, no external access
- **No Authentication**: Assumes trusted LAN
- **No Encryption**: WiFi encryption is recommended
- **Firewall**: Configure to allow ports 5000, 5443, 5353

## 📞 Getting Help

### Check Logs
**Desktop:**
```
Console output from `cargo tauri dev`
Look for [Server], [Stream], [WebSocket] messages
```

**Mobile:**
```
Android Studio → Logcat
Filter: "vibeon" or "MusicStreamClient"
```

### Common Log Messages

✅ **Good:**
```
[Server] HTTP/WS listening on http://0.0.0.0:5000
✅ Loaded 1234 tracks (total: 1234)
📀 Now playing: Song Title by Artist
📋 Queue updated with 50 tracks
```

❌ **Problems:**
```
❌ Failed to browse library: 503
  → Desktop server not running or database error
  
❌ Could not resolve IPv4 address
  → mDNS resolution failed, use manual IP
  
❌ Failed to get track metadata
  → File path invalid or disk error on desktop
```

## 🎯 Quick Reference

| Task | Steps |
|------|-------|
| **Start Streaming** | Desktop: Start Server → Mobile: Tap Device → Browse → Play |
| **Play Specific Song** | Mobile: Search → Tap Song → Plays on desktop + mobile |
| **Control Queue** | Desktop: Edit queue → Mobile: See updates in real-time |
| **Skip Track** | Mobile: Tap next/prev button or swipe gesture |
| **Seek in Song** | Mobile: Tap on progress bar to jump to time |
| **Change Volume** | Mobile: Use system volume buttons |
| **Stop Streaming** | Mobile: Tap stop or go back to discovery |

## 🚨 Emergency Troubleshooting

**"App Crashes on Launch"**
```
1. Uninstall: adb uninstall moe.memesta.vibeon
2. Clear cache: adb shell pm clear moe.memesta.vibeon
3. Reinstall app
```

**"Desktop Can't Index Music"**
```
1. Check folder permissions (music folder readable)
2. Check disk space (at least 1GB free)
3. Try smaller folder first to test
4. Check file format supported (mp3, flac, aac, ogg, opus, wav)
```

**"WebSocket Disconnects Randomly"**
```
1. Check WiFi stability (router signal good?)
2. Update router firmware
3. Restart router and devices
4. Try 5GHz WiFi if available
```

---

That's it! You're ready to enjoy local music streaming. 🎵

For detailed documentation, see:
- `STREAMING_IMPLEMENTATION.md` - Architecture & design
- `STREAMING_API.md` - API reference
- `ANDROID_STREAMING_INTEGRATION.md` - Developer guide
