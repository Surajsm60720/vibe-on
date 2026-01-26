# Bundling Essentia Analyzer with Vibe-On

This guide explains how to bundle the Python audio analyzer (Essentia) with your Tauri app so users don't need to install Python or Essentia manually.

## Overview

We use **PyInstaller** to create a standalone executable that includes:
- Python interpreter
- Essentia library
- All dependencies (numpy, etc.)
- The `analyze_track.py` script

## Step 1: Build the Standalone Executable

### Prerequisites

Install PyInstaller and Essentia:

```bash
# macOS
brew install essentia
pip3 install essentia pyinstaller

# Linux
pip3 install essentia pyinstaller

# Windows
pip3 install essentia pyinstaller
# Note: Windows may require Visual Studio Build Tools
```

### Build Command

```bash
cd src-tauri/sidecar
python3 build_analyzer.py
```

This creates:
- `dist/analyze_track` (macOS/Linux)
- `dist/analyze_track.exe` (Windows)

The executable will be **60-150 MB** depending on platform.

## Step 2: Configure Tauri to Bundle the Executable

### Update `tauri.conf.json`

Add the sidecar configuration:

```json
{
  "bundle": {
    "externalBin": [
      "sidecar/analyze_track"
    ],
    "resources": [
      "sidecar/analyze_track",
      "sidecar/analyze_track.exe"
    ]
  }
}
```

Tauri will automatically:
- Bundle the correct platform executable
- Add proper permissions
- Place it in the resources directory

### Platform-Specific Notes

**macOS:**
- The binary will be signed during build
- No additional setup needed

**Windows:**
- May need to whitelist in antivirus
- Larger executable size (~150MB)

**Linux:**
- Set executable permission: `chmod +x dist/analyze_track`
- AppImage includes the binary automatically

## Step 3: Update Rust Code

Modify `src-tauri/src/mood/analyzer.rs` to use the bundled sidecar:

```rust
use tauri::Manager;

impl AudioAnalyzer {
    pub fn new(app_handle: &tauri::AppHandle) -> Result<Self, String> {
        // Get sidecar binary from Tauri
        let sidecar_path = app_handle
            .path()
            .resolve("analyze_track", tauri::path::BaseDirectory::Resource)
            .map_err(|e| format!("Failed to resolve sidecar path: {}", e))?;
        
        if !sidecar_path.exists() {
            return Err(format!(
                "Analyzer executable not found at: {}",
                sidecar_path.display()
            ));
        }
        
        Ok(Self { sidecar_path })
    }
    
    pub fn analyze_track(&self, audio_path: &str) -> Result<AudioFeatures, String> {
        // Use the bundled executable directly (no python3.11 needed)
        let output = Command::new(&self.sidecar_path)
            .arg(audio_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to run analyzer: {}", e))?;
        
        // ... rest of the code
    }
}
```

## Step 4: Update Initialization

In your Tauri command initialization, pass the app handle:

```rust
#[tauri::command]
pub fn init_mood_features(app: tauri::AppHandle) -> Result<(), String> {
    let analyzer = AudioAnalyzer::new(&app)?;
    // Store analyzer in state...
    Ok(())
}
```

## Step 5: Build Your App

```bash
# Development (still uses Python directly)
npm run tauri dev

# Production build with bundled analyzer
npm run tauri build
```

## Testing the Bundled Version

1. Build the app: `npm run tauri build`
2. Install the app from `src-tauri/target/release/bundle/`
3. Try mood analysis features
4. Check that it works without Python installed

## Troubleshooting

### "Analyzer executable not found"
- Ensure `build_analyzer.py` completed successfully
- Check that the binary is in `src-tauri/sidecar/dist/`
- Verify `tauri.conf.json` has correct paths

### "Permission denied" (macOS/Linux)
```bash
chmod +x src-tauri/sidecar/dist/analyze_track
```

### Large Bundle Size
- This is normal (60-150 MB for the analyzer)
- Consider compressing with UPX: `upx --best dist/analyze_track`
- Trade-off: larger app vs. no user setup

### Windows Antivirus Flags
- PyInstaller executables sometimes trigger false positives
- Sign your app with a code signing certificate
- Report false positive to antivirus vendor

## Development vs Production

### Development Mode
Keep using Python directly for faster iteration:
- No need to rebuild analyzer executable
- Easier debugging
- Faster dev cycle

### Production Mode
Use bundled executable:
- No Python dependency
- Users can't see/modify code
- Better UX (no manual setup)

## Alternative: Pre-built Binaries

Instead of users building, you can:
1. Build analyzer on each platform
2. Commit to git: `src-tauri/sidecar/bin/{platform}/analyze_track`
3. Configure Tauri to use platform-specific path

This way your CI/CD builds the final app without needing Essentia.

## CI/CD Setup

For GitHub Actions or similar:

```yaml
- name: Install Essentia
  run: |
    # macOS
    brew install essentia
    pip3 install essentia pyinstaller
    
- name: Build Analyzer
  run: |
    cd src-tauri/sidecar
    python3 build_analyzer.py
    
- name: Build Tauri App
  run: npm run tauri build
```

---

**Result:** Users can now install your app without any Python/Essentia setup! 🎉
