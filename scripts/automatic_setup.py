#!/usr/bin/env python3
"""
Automatic Setup and Installer for AE-ENGINE
Installs all dependencies and configures the project automatically
"""

import os
import sys
import subprocess
import platform
import urllib.request
import zipfile
import shutil
from pathlib import Path

PROJECT_ROOT = Path(r'd:\rj\wasteland_project')
GODOT_VERSION = "4.6"
GODOT_URLS = [
    "https://downloads.tuxfamily.org/godotengine/4.6/Godot_v4.6-stable_win64.exe.zip",
    "https://github.com/godotengine/godot/releases/download/4.6-stable/Godot_v4.6-stable_win64.exe.zip",
]

def download_file(url, dest_path):
    """Download a file with progress"""
    print(f"Downloading: {url}")
    
    def report_progress(block_num, block_size, total_size):
        if total_size > 0:
            downloaded = block_num * block_size
            percent = min(100, (downloaded / total_size) * 100)
            sys.stdout.write(f"\rProgress: {percent:.1f}% [{downloaded//1024}KB / {total_size//1024}KB]")
            sys.stdout.flush()
    
    try:
        urllib.request.urlretrieve(url, dest_path, reporthook=report_progress)
        print("\nDownload completed!")
        return True
    except Exception as e:
        print(f"\nDownload failed: {e}")
        return False

def install_godot():
    """Try to install Godot 4.6"""
    print("\n" + "="*60)
    print("  INSTALLING GODOT ENGINE")
    print("="*60)
    
    bin_dir = PROJECT_ROOT / "bin"
    bin_dir.mkdir(exist_ok=True)
    
    # Check if Godot already exists
    godot_exe = bin_dir / "godot.exe"
    if godot_exe.exists():
        print(f"✓ Godot already installed: {godot_exe}")
        return str(godot_exe)
    
    # Try to download
    zip_path = bin_dir / "godot.zip"
    for url in GODOT_URLS:
        try:
            if download_file(url, zip_path):
                if zipfile.is_zipfile(zip_path):
                    print("Extracting Godot...")
                    with zipfile.ZipFile(zip_path, 'r') as zip_ref:
                        zip_ref.extractall(bin_dir)
                    
                    # Find the exe
                    for f in bin_dir.glob("*.exe"):
                        if "godot" in f.name.lower():
                            shutil.move(str(f), str(godot_exe))
                            print(f"✓ Godot installed: {godot_exe}")
                            return str(godot_exe)
                
                zip_path.unlink(missing_ok=True)
        except Exception as e:
            print(f"Failed to download from {url}: {e}")
            continue
    
    # Try winget
    print("\nTrying winget...")
    try:
        result = subprocess.run(
            ["winget", "install", "GodotEngine.GodotEngine.4.6", "-e", "--accept-source-agreements"],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            print("✓ Godot installed via winget")
            return "godot.exe"
    except Exception as e:
        print(f"winget failed: {e}")
    
    return None

def build_gdextension():
    """Build Rust GDExtension"""
    print("\n" + "="*60)
    print("  BUILDING GDEXTENSION")
    print("="*60)
    
    # Check if DLL exists
    target_dll = PROJECT_ROOT / "godot_project" / "bin" / "wasteland_gdextension.dll"
    if target_dll.exists():
        print("✓ GDExtension DLL already exists")
        return True
    
    # Check cargo
    try:
        subprocess.run(["cargo", "--version"], check=True, capture_output=True)
    except Exception as e:
        print(f"✗ Cargo not found: {e}")
        return False
    
    # Build
    print("Building GDExtension (release mode)...")
    os.chdir(PROJECT_ROOT)
    
    try:
        result = subprocess.run(
            ["cargo", "build", "--release", "-p", "gdextension"],
            capture_output=True, text=True
        )
        
        if result.returncode == 0:
            # Copy to target
            source_dll = PROJECT_ROOT / "target" / "release" / "wasteland_gdextension.dll"
            if source_dll.exists():
                target_dir = target_dll.parent
                target_dir.mkdir(exist_ok=True)
                shutil.copy2(source_dll, target_dll)
                print(f"✓ GDExtension built: {target_dll}")
                return True
            else:
                # Try gdextension target
                alt_source = PROJECT_ROOT / "gdextension" / "target" / "release" / "wasteland_gdextension.dll"
                if alt_source.exists():
                    shutil.copy2(alt_source, target_dll)
                    print(f"✓ GDExtension built: {target_dll}")
                    return True
        
        print(f"✗ Build failed: {result.stderr}")
        return False
    except Exception as e:
        print(f"✗ Build error: {e}")
        return False

def update_launch_script(godot_path):
    """Update launch script with correct Godot path"""
    print("\nUpdating launch script...")
    
    bat_content = f"""@echo off
setlocal enabledelayedexpansion

set "GODOT_EXE={godot_path}"
set "PROJECT_DIR=%~dp0godot_project"
set "OUTPUT_LOG=%~dp0logs\\game.log"

if not exist "%~dp0logs" mkdir "%~dp0logs"

echo [LAUNCHER] Starting AE-ENGINE...
echo [LAUNCHER] Godot: %GODOT_EXE%
echo [LAUNCHER] Project: %PROJECT_DIR%
echo [LAUNCHER] Log: %OUTPUT_LOG%

if exist "%GODOT_EXE%" (
    "%GODOT_EXE%" --path "%PROJECT_DIR%"
) else (
    echo [ERROR] Godot not found!
    echo Please install Godot 4.6 first.
    pause
)
pause
"""
    
    with open(PROJECT_ROOT / "launch_game.bat", 'w') as f:
        f.write(bat_content)
    
    print("✓ Launch script updated")

def write_setup_guide():
    """Write setup guide"""
    guide_content = """# AE-ENGINE Setup Guide

## Quick Start

### If automatic setup succeeded:
1. Run `launch_game.bat`

### If you need to install manually:

## Step 1: Install Godot

### Option A: Download from website
- Go to: https://godotengine.org/download/windows/
- Download: Godot_v4.6-stable_win64.exe
- Save it as `d:\\rj\\wasteland_project\\bin\\godot.exe`

### Option B: Use winget (Windows Package Manager)
```powershell
winget install GodotEngine.GodotEngine.4.6 -e
```

## Step 2: Build GDExtension

If you have Rust installed:

```powershell
cd d:\rj\wasteland_project
cargo build --release
copy target\release\wasteland_gdextension.dll godot_project\bin\
```

## Step 3: Launch the Game

Double-click `launch_game.bat` or run:
```powershell
godot --path godot_project
```

## In-Game Controls

- **WASD**: Move
- **Space**: Jump
- **T**: Run Tests
- **P**: Pause
- **F9**: Performance Stats
- **ESC**: Quit

## Test the Game

Press **T** key in-game to run all tests, which will verify:
- ✅ World Generation
- ✅ NPC Spawning
- ✅ Animal System
- ✅ Performance
- ✅ Memory Stability
"""
    
    with open(PROJECT_ROOT / "SETUP_GUIDE.md", 'w', encoding='utf-8') as f:
        f.write(guide_content)
    
    print("✓ Setup guide written")

def main():
    print("="*60)
    print("  AE-ENGINE - AUTOMATIC SETUP")
    print("="*60)
    
    # Step 1: Godot
    godot_path = install_godot()
    
    # Step 2: Build GDExtension
    build_gdextension()
    
    # Step 3: Update launch script
    if godot_path:
        update_launch_script(godot_path)
    
    # Step 4: Write guide
    write_setup_guide()
    
    # Summary
    print("\n" + "="*60)
    print("  SETUP COMPLETE")
    print("="*60)
    
    # Check final status
    print("\nSTATUS:")
    gd_dll = PROJECT_ROOT / "godot_project" / "bin" / "wasteland_gdextension.dll"
    if gd_dll.exists():
        print("  ✓ GDExtension: READY")
    else:
        print("  ⚠ GDExtension: Needs build")
    
    if godot_path and Path(godot_path).exists():
        print(f"  ✓ Godot: READY ({godot_path})")
        print("\nNext step: Double-click launch_game.bat")
    else:
        print("  ⚠ Godot: Please install manually")
        print("\nSee SETUP_GUIDE.md for details")
    
    print("\n" + "="*60)
    return 0

if __name__ == "__main__":
    sys.exit(main())