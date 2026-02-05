# Mobile Connection Troubleshooting Guide

## Error: "Failed to load library from server"

This error means your Android phone can't connect to your PC's VIBE-ON server. Follow these steps to fix it:

### ✅ Step 1: Check PC Server is Running

1. **Open VIBE-ON on your PC**
2. **Click the Mobile icon** in the player bar (bottom of screen)
3. **Check if server is started** - You should see:
   - "Server Address: `192.168.x.x:5000`" displayed
   - A green indicator showing server is running

If server is not running:
- Click "Start Server" button
- Wait for server address to appear

### ✅ Step 2: Check Network Connection

**Both devices MUST be on the same WiFi network!**

**On PC:**
1. Check WiFi connection in system tray
2. Note your WiFi network name (SSID)
3. Note your IP address from VIBE-ON mobile popup

**On Android:**
1. Open Settings → WiFi
2. Verify connected to SAME WiFi as PC
3. NOT cellular data, NOT mobile hotspot

**Common Issues:**
- ❌ PC on 5GHz WiFi, phone on 2.4GHz → Switch both to same band
- ❌ PC on Ethernet, phone on WiFi → Both need same network
- ❌ Guest WiFi network → May block device-to-device communication

### ✅ Step 3: Manual Connection

If automatic discovery doesn't work, connect manually:

1. **Get PC's IP Address:**
   - Open VIBE-ON mobile popup on PC
   - Note the "Server Address" (e.g., `192.168.1.100:5000`)

2. **On Android App:**
   - Discovery screen → scroll down
   - Find "Manual Server Entry" section
   - Enter IP: `192.168.1.100` (your PC's IP)
   - Enter Port: `5000` (default)
   - Click "Connect"

### ✅ Step 4: Check Firewall Settings

**Windows Firewall:**

1. Open Windows Security → Firewall & Network Protection
2. Click "Allow an app through firewall"
3. Find "VIBE-ON" in the list
4. Check both "Private" and "Public" boxes
5. Click OK

**Linux Firewall (ufw):**
```bash
sudo ufw allow 5000/tcp
sudo ufw status
```

**For testing, temporarily disable firewall:**
- Windows: Turn off Windows Defender Firewall (test only!)
- Linux: `sudo ufw disable` (test only!)
- Remember to re-enable after testing!

### ✅ Step 5: Check Port Availability

**On PC, open terminal:**

**Windows (PowerShell):**
```powershell
netstat -ano | findstr :5000
```

**Linux/Mac:**
```bash
sudo lsof -i :5000
# OR
sudo netstat -tulpn | grep 5000
```

You should see VIBE-ON using port 5000. If another app is using it, change port in VIBE-ON settings.

### ✅ Step 6: Test Connection Manually

**From Android, test if PC is reachable:**

1. **Install a network scanner app** (e.g., "Fing", "Network Analyzer")
2. **Scan network** for devices
3. **Find your PC** in the list
4. **Note the IP address**

**Or use your browser:**
1. Open browser on Android
2. Type: `http://192.168.x.x:5000/api/info` (use your PC's IP)
3. You should see JSON response with server info

### ✅ Step 7: Check Android App Logs

**Enable USB debugging and check logs:**

1. **Connect phone to PC via USB**
2. **Enable Developer Options and USB Debugging**
3. **Run:**
   ```bash
   adb logcat | grep -E "Vibeon|MusicStreamClient|LibraryViewModel"
   ```
4. **Look for connection errors** in output

**Common log messages:**
- `ConnectException` → Firewall blocking or server not running
- `UnknownHostException` → Wrong IP address
- `SocketTimeoutException` → Network too slow or server overloaded

### ✅ Step 8: Router Configuration

**Some routers block device-to-device communication:**

1. **Access router admin panel** (usually `192.168.1.1` or `192.168.0.1`)
2. **Look for:**
   - "AP Isolation" → Disable it
   - "Client Isolation" → Disable it
   - "Wireless Isolation" → Disable it
3. **Save settings and reboot router**

### ✅ Step 9: Use Different Network

**Test with phone hotspot:**

1. **On phone:** Enable mobile hotspot
2. **On PC:** Connect to phone's hotspot WiFi
3. **Check PC's new IP** in VIBE-ON mobile popup
4. **On phone:** Turn off hotspot, connect to PC's hotspot network
5. **Try connecting** with new IP

This helps determine if it's a router issue.

### 📋 Diagnostic Checklist

Print this and check off each item:

- [ ] PC VIBE-ON app is running
- [ ] Mobile server is started (green indicator)
- [ ] Both devices on SAME WiFi network
- [ ] IP address is correct (from PC mobile popup)
- [ ] Port 5000 is correct
- [ ] Firewall allows VIBE-ON
- [ ] No VPN or proxy running
- [ ] Router doesn't have AP isolation
- [ ] Android app has network permissions
- [ ] Can ping PC from phone (network scanner app)

### 🔍 Advanced Debugging

**Check if HTTP server is responding:**

**From PC terminal:**
```bash
curl http://localhost:5000/api/info
```

Should return:
```json
{
  "name": "VIBE-ON Server",
  "version": "1.0.0",
  "librarySize": 1234,
  "localIp": "192.168.1.100"
}
```

**From another PC on same network:**
```bash
curl http://192.168.1.100:5000/api/info
```

If this works but phone doesn't connect → Phone firewall or app permissions issue

### 📱 Android Permissions

Make sure app has required permissions:

1. Settings → Apps → VIBE-ON
2. Permissions → check:
   - ✅ Network access
   - ✅ WiFi state
   - ✅ Internet
3. If any are missing, grant them

### 🔄 Last Resort: Fresh Start

1. **PC:** Stop server, close VIBE-ON, restart PC
2. **Android:** Force stop app, clear cache (not data), restart phone
3. **Network:** Reboot router
4. **Try again** with manual IP entry

### 💡 Common Error Messages Explained

| Error | Cause | Solution |
|-------|-------|----------|
| "Connection refused" | Server not running or firewall | Start server, check firewall |
| "Unknown host" | Wrong IP address | Verify IP from PC app |
| "Connection timeout" | Different networks or router blocking | Check WiFi, disable AP isolation |
| "Failed to load library" | Generic connection failure | Follow all steps above |

### 📞 Still Not Working?

Check logs and provide this info:
- PC IP address
- Android IP address (Settings → WiFi → Advanced)
- Router model
- PC OS and version
- Android version
- Error message from Android logs

**Logs to collect:**

**PC logs:**
```bash
# From vibe-on directory
cat src-tauri/output.txt | grep -i "server\|mobile\|websocket"
```

**Android logs:**
```bash
adb logcat -d | grep -E "Vibeon|MusicStream|WebSocket" > android_logs.txt
```

Share these logs for debugging support.
