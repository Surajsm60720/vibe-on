# Windows Setup Guide for Vibe-On

Hey! Thanks for helping test on Windows. This is a **one-time setup** that takes about 5-10 minutes.

## What This Does

This setup builds a standalone audio analyzer that bundles Python + Essentia into a single `.exe` file. After this runs, you'll **never need to install Python or Essentia again** - the app will just work!

## Prerequisites

You need **Python 3.10 or newer** installed:
- Download from: https://www.python.org/downloads/
- ⚠️ **IMPORTANT**: Check "Add Python to PATH" during installation!

## Quick Start

1. **Pull the latest code:**
   ```bash
   git pull
   ```

2. **Run the setup script:**
   ```bash
   setup-windows.bat
   ```
   
3. **Wait for it to finish** (5-10 minutes)
   - It will install PyInstaller and Essentia
   - It will build the standalone analyzer
   - You'll see "SUCCESS!" when done

4. **That's it!** You can now:
   ```bash
   npm run tauri dev     # Run in development
   npm run tauri build   # Build the installer
   ```

## What If It Fails?

### "Python not found"
- Install Python from https://www.python.org/downloads/
- Make sure to check "Add Python to PATH"
- Restart your terminal and try again

### "Essentia installation failed"
Windows sometimes has trouble with Essentia. Try this alternative:
```bash
pip install essentia-tensorflow
```

Or install Visual Studio Build Tools:
https://visualstudio.microsoft.com/downloads/

### "Build failed"
- Check that you have enough disk space (need ~500MB)
- Try running as Administrator
- Check the error messages - they usually tell you what's missing

## After Setup

Once the setup completes successfully:
- The app will work without Python/Essentia installed
- You can even uninstall Python if you want
- The standalone analyzer is at: `src-tauri/sidecar/dist/analyze_track.exe`
- Future builds will automatically include it

## Questions?

If you run into issues, send me:
1. The error messages from the setup script
2. Your Python version: `python --version`
3. Your Windows version

Thanks for helping! 🙏
