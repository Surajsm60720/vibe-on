@echo off
REM Windows One-Time Setup for Vibe-On
REM This builds a standalone analyzer so you never need Python/Essentia again!

echo ========================================
echo   Vibe-On Windows Setup
echo ========================================
echo.
echo This will set up everything needed for Vibe-On.
echo You only need to run this ONCE.
echo.
pause

echo.
echo [1/5] Checking Python...
python --version >nul 2>&1
if errorlevel 1 (
    echo.
    echo ERROR: Python not found!
    echo.
    echo Please install Python 3.10+ from: https://www.python.org/downloads/
    echo Make sure to check "Add Python to PATH" during installation!
    echo.
    pause
    exit /b 1
)
python --version
echo OK: Python found

echo.
echo [2/5] Upgrading pip...
python -m pip install --upgrade pip --quiet

echo.
echo [3/5] Installing dependencies (this may take a few minutes)...
echo Installing PyInstaller...
python -m pip install pyinstaller --quiet
echo Installing Essentia (this is the big one, please wait)...
python -m pip install essentia numpy --quiet
if errorlevel 1 (
    echo.
    echo WARNING: Essentia installation failed!
    echo.
    echo On Windows, Essentia may require Visual Studio Build Tools.
    echo Try: pip install essentia-tensorflow (lighter alternative)
    echo Or download from: https://visualstudio.microsoft.com/downloads/
    echo.
    pause
    exit /b 1
)
echo OK: All dependencies installed

echo.
echo [4/5] Building standalone analyzer executable...
cd src-tauri\sidecar
python build_analyzer.py
cd ..\..

if not exist "src-tauri\sidecar\dist\analyze_track.exe" (
    echo.
    echo ERROR: Build failed!
    echo Check the output above for errors.
    pause
    exit /b 1
)

echo.
echo [5/5] Cleaning up...
echo OK: Setup complete!

echo.
echo ========================================
echo   SUCCESS! 
echo ========================================
echo.
echo The standalone analyzer is ready at:
echo   src-tauri\sidecar\dist\analyze_track.exe
echo.
echo You can now:
echo   1. Run: npm run tauri dev (for development)
echo   2. Run: npm run tauri build (to create installer)
echo.
echo The app will work WITHOUT needing Python/Essentia installed!
echo.
echo NOTE: You can uninstall Python and Essentia now if you want,
echo       the app no longer needs them.
echo.
pause
