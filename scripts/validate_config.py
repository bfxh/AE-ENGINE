#!/usr/bin/env python3
"""
AE-ENGINE Configuration Validator
Checks all configuration files and dependencies
"""

import os
import sys
import json
import toml
from pathlib import Path

PROJECT_ROOT = Path(r'd:\rj\wasteland_project')

def check_config():
    """Check all project configurations"""
    print("=" * 60)
    print("  AE-ENGINE CONFIGURATION VALIDATOR")
    print("=" * 60)
    
    checks = []
    
    # 1. Check config.toml
    print("\n[1] Checking config.toml...")
    config_path = PROJECT_ROOT / "config.toml"
    if config_path.exists():
        try:
            config = toml.load(config_path)
            print(f"  ✓ Loaded successfully (version: {config.get('game', {}).get('version', 'unknown')})")
            checks.append(("config.toml", True, "Loaded successfully"))
        except Exception as e:
            print(f"  ✗ Failed to parse: {e}")
            checks.append(("config.toml", False, f"Parse error: {e}"))
    else:
        print(f"  ✗ Not found: {config_path}")
        checks.append(("config.toml", False, "File not found"))
    
    # 2. Check project.godot
    print("\n[2] Checking project.godot...")
    project_godot = PROJECT_ROOT / "godot_project" / "project.godot"
    if project_godot.exists():
        print(f"  ✓ Found: {project_godot}")
        checks.append(("project.godot", True, "Found"))
        
        # Parse basic structure
        with open(project_godot, 'r') as f:
            content = f.read()
            if 'config_version=5' in content:
                print("  ✓ config_version=5 (Godot 4.x compatible)")
            if 'Wasteland' in content:
                print("  ✓ Project name: Wasteland")
    else:
        print(f"  ✗ Not found: {project_godot}")
        checks.append(("project.godot", False, "File not found"))
    
    # 3. Check GDExtension
    print("\n[3] Checking GDExtension...")
    gdextension_file = PROJECT_ROOT / "godot_project" / "wasteland.gdextension"
    dll_path = PROJECT_ROOT / "godot_project" / "bin" / "wasteland_gdextension.dll"
    
    if gdextension_file.exists():
        print(f"  ✓ Config file: {gdextension_file}")
        checks.append(("wasteland.gdextension", True, "Found"))
    else:
        print(f"  ✗ Config file missing: {gdextension_file}")
        checks.append(("wasteland.gdextension", False, "File not found"))
    
    if dll_path.exists():
        print(f"  ✓ DLL found: {dll_path}")
        checks.append(("wasteland_gdextension.dll", True, "Found"))
    else:
        print(f"  ✗ DLL missing: {dll_path}")
        checks.append(("wasteland_gdextension.dll", False, "File not found"))
    
    # 4. Check main scene
    print("\n[4] Checking main scene...")
    main_scene = PROJECT_ROOT / "godot_project" / "scenes" / "main.tscn"
    if main_scene.exists():
        print(f"  ✓ Found: {main_scene}")
        checks.append(("main.tscn", True, "Found"))
    else:
        print(f"  ✗ Not found: {main_scene}")
        checks.append(("main.tscn", False, "File not found"))
    
    # 5. Check scripts
    print("\n[5] Checking scripts...")
    scripts_dir = PROJECT_ROOT / "godot_project" / "scripts"
    if scripts_dir.exists():
        gd_scripts = list(scripts_dir.glob("*.gd"))
        print(f"  ✓ {len(gd_scripts)} GDScript files found")
        checks.append(("scripts", True, f"{len(gd_scripts)} files"))
        
        # Check key scripts
        key_scripts = ["wasteland_main.gd", "wasteland_forest_generator.gd", "game_manager.gd"]
        for script in key_scripts:
            script_path = scripts_dir / script
            if script_path.exists():
                print(f"    ✓ {script}")
            else:
                print(f"    ✗ {script}")
    else:
        print(f"  ✗ Scripts directory missing: {scripts_dir}")
        checks.append(("scripts", False, "Directory not found"))
    
    # 6. Check Rust crates
    print("\n[6] Checking Rust configuration...")
    cargo_toml = PROJECT_ROOT / "Cargo.toml"
    if cargo_toml.exists():
        print("  ✓ Cargo.toml found")
        checks.append(("Cargo.toml", True, "Found"))
        
        # Check build artifacts
        release_dll = PROJECT_ROOT / "target" / "release" / "wasteland_gdextension.dll"
        if release_dll.exists():
            print("  ✓ Release build found")
            checks.append(("Release DLL", True, "Found"))
        else:
            print("  ⚠ Release build missing")
            checks.append(("Release DLL", False, "Not built"))
    else:
        print("  ✗ Cargo.toml not found")
        checks.append(("Cargo.toml", False, "File not found"))
    
    # 7. Check launch script
    print("\n[7] Checking launch script...")
    launch_bat = PROJECT_ROOT / "launch_game.bat"
    if launch_bat.exists():
        print(f"  ✓ Found: {launch_bat}")
        checks.append(("launch_game.bat", True, "Found"))
    else:
        print(f"  ✗ Not found: {launch_bat}")
        checks.append(("launch_game.bat", False, "File not found"))
    
    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    
    passed = sum(1 for _, status, _ in checks if status)
    total = len(checks)
    
    print(f"\nPassed: {passed}/{total}")
    
    if passed == total:
        print("\n✓ ALL CHECKS PASSED")
        print("\nTo run the game:")
        print("  1. Install Godot 4.6 and add to PATH")
        print("  2. Run: launch_game.bat")
        print("  3. Or run: godot --path godot_project")
    else:
        print("\n⚠ SOME CHECKS FAILED")
        print("\nFailed checks:")
        for name, status, msg in checks:
            if not status:
                print(f"  - {name}: {msg}")
    
    print("\n" + "=" * 60)
    
    return passed == total

def find_godot():
    """Try to find Godot installation"""
    print("\n[SEARCHING FOR GODOT INSTALLATION...]")
    
    search_paths = [
        "C:\\Program Files\\Godot Engine",
        "D:\\Program Files\\Godot Engine",
        "C:\\Program Files (x86)\\Godot Engine",
        os.path.join(os.environ.get('USERPROFILE', ''), 'AppData', 'Local', 'Programs', 'Godot'),
        str(PROJECT_ROOT / "bin"),
    ]
    
    found = []
    for path in search_paths:
        if os.path.exists(path):
            for root, dirs, files in os.walk(path):
                for f in files:
                    if f.lower().startswith("godot") and f.lower().endswith(".exe"):
                        found.append(os.path.join(root, f))
                # Limit depth
                if len(root.split(os.sep)) - len(path.split(os.sep)) > 2:
                    dirs.clear()
    
    if found:
        print("\nFound Godot installations:")
        for i, gd in enumerate(found, 1):
            print(f"  {i}. {gd}")
        return found[0]
    else:
        print("No Godot installation found.")
        print("\nPlease download and install Godot 4.6 from:")
        print("  https://godotengine.org/download/windows/")
        return None

if __name__ == "__main__":
    check_config()
    find_godot()