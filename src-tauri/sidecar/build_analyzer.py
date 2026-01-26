#!/usr/bin/env python3
"""
Build standalone analyzer executable using PyInstaller.
This bundles Python + Essentia + all dependencies into a single executable.

Usage:
    python build_analyzer.py

This will create:
    - dist/analyze_track (macOS/Linux)
    - dist/analyze_track.exe (Windows)
"""

import os
import sys
import shutil
import subprocess
from pathlib import Path

def main():
    script_dir = Path(__file__).parent
    os.chdir(script_dir)
    
    print("🔨 Building standalone audio analyzer...")
    print(f"📁 Working directory: {script_dir}")
    
    # Check if PyInstaller is installed
    try:
        import PyInstaller
        print(f"✅ PyInstaller {PyInstaller.__version__} found")
    except ImportError:
        print("❌ PyInstaller not found. Installing...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "pyinstaller"])
        print("✅ PyInstaller installed")
    
    # Check if Essentia is installed
    try:
        import essentia
        print(f"✅ Essentia found")
    except ImportError:
        print("❌ Essentia not found!")
        print("Please install it first:")
        print("  macOS: brew install essentia && pip install essentia")
        print("  Linux: pip install essentia")
        print("  Windows: pip install essentia (requires Visual Studio)")
        sys.exit(1)
    
    # Clean previous build
    for dir_name in ['build', 'dist', '__pycache__']:
        dir_path = script_dir / dir_name
        if dir_path.exists():
            print(f"🧹 Cleaning {dir_name}/")
            shutil.rmtree(dir_path)
    
    # Build command
    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--onefile",  # Single executable
        "--name", "analyze_track",
        "--clean",
        "--noconfirm",
        # Hidden imports that PyInstaller might miss
        "--hidden-import", "essentia",
        "--hidden-import", "essentia.standard",
        "--hidden-import", "numpy",
        "--hidden-import", "json",
        # Optimize
        "--strip",  # Strip debug symbols (smaller binary)
        "analyze_track.py"
    ]
    
    print("\n🚀 Running PyInstaller...")
    print(f"Command: {' '.join(cmd)}")
    
    result = subprocess.run(cmd, capture_output=False)
    
    if result.returncode != 0:
        print("\n❌ Build failed!")
        sys.exit(1)
    
    # Check output
    exe_name = "analyze_track.exe" if sys.platform == "win32" else "analyze_track"
    exe_path = script_dir / "dist" / exe_name
    
    if not exe_path.exists():
        print(f"\n❌ Executable not found at {exe_path}")
        sys.exit(1)
    
    size_mb = exe_path.stat().st_size / (1024 * 1024)
    print(f"\n✅ Build successful!")
    print(f"📦 Executable: {exe_path}")
    print(f"💾 Size: {size_mb:.2f} MB")
    
    # Test the executable
    print("\n🧪 Testing executable...")
    test_result = subprocess.run([str(exe_path), "--version"], capture_output=True)
    if test_result.returncode != 0:
        print("⚠️  Note: Executable test returned non-zero (this is expected if --version not implemented)")
    else:
        print("✅ Executable test passed")
    
    print("\n📋 Next steps:")
    print(f"1. Copy {exe_path} to your Tauri resources")
    print("2. Update tauri.conf.json to bundle it")
    print("3. Update analyzer.rs to use the bundled executable")
    print("\nSee BUNDLING.md for detailed instructions.")

if __name__ == "__main__":
    main()
