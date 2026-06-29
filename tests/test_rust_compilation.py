"""
Rust Compilation Test
Verifies that all Rust crates compile successfully
"""
import subprocess
import sys
import os

def run_cargo_check():
    """Check if cargo is available and run cargo check."""
    print("[TEST] Cargo Check")
    
    try:
        result = subprocess.run(
            ["cargo", "--version"],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        if result.returncode != 0:
            print("  SKIP: cargo not available in PATH")
            return True
            
        os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        
        result = subprocess.run(
            ["cargo", "check", "--workspace", "--all-targets"],
            capture_output=True,
            text=True,
            timeout=300
        )
        
        if result.returncode == 0:
            print("  PASS: All crates compile successfully")
            return True
        else:
            print(f"  FAIL: Compilation errors:\n{result.stderr[:2000]}")
            return False
    except FileNotFoundError:
        print("  SKIP: cargo executable not found")
        return True

def verify_gdextension_binary():
    """Verify that GDExtension binaries exist."""
    print("[TEST] GDExtension Binary Verification")
    
    target_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "target")
    
    binaries = [
        os.path.join(target_dir, "debug", "wasteland_gdextension.dll"),
        os.path.join(target_dir, "release", "wasteland_gdextension.dll"),
    ]
    
    all_found = True
    for binary in binaries:
        if os.path.exists(binary):
            size = os.path.getsize(binary)
            print(f"  Found: {os.path.basename(binary)} ({size:,} bytes)")
        else:
            print(f"  MISSING: {binary}")
            all_found = False
    
    if all_found:
        print("  PASS: All binaries present")
    return all_found

def verify_godot_project():
    """Verify Godot project structure."""
    print("[TEST] Godot Project Verification")
    
    godot_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "godot_project")
    
    required_files = [
        "project.godot",
        "wasteland.gdextension",
        "bin/wasteland_gdextension.dll",
        "scripts/terrain_generator.gd",
        "scripts/building_generator.gd",
        "scripts/l_system_tree.gd",
        "scripts/voxel_mesh_renderer.gd",
        "scripts/wasteland_controller.gd",
        "scenes/main.tscn",
    ]
    
    all_found = True
    for file in required_files:
        path = os.path.join(godot_dir, file)
        if os.path.exists(path):
            print(f"  Found: {file}")
        else:
            print(f"  MISSING: {file}")
            all_found = False
    
    if all_found:
        print("  PASS: All required files present")
    return all_found

def verify_blender_plugin():
    """Verify Blender plugin installation."""
    print("[TEST] Blender Plugin Verification")
    
    plugin_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "blender_plugin")
    
    required_files = [
        "wasteland_generator.py",
        "install_plugin.py",
        "verify_wasteland.py",
        "install_report.json",
    ]
    
    all_found = True
    for file in required_files:
        path = os.path.join(plugin_dir, file)
        if os.path.exists(path):
            print(f"  Found: {file}")
        else:
            print(f"  MISSING: {file}")
            all_found = False
    
    if all_found:
        print("  PASS: All plugin files present")
    return all_found

def main():
    print("=" * 60)
    print("RUST COMPILATION & PROJECT VERIFICATION")
    print("=" * 60)
    
    tests = [
        run_cargo_check,
        verify_gdextension_binary,
        verify_godot_project,
        verify_blender_plugin,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        if test():
            passed += 1
        else:
            failed += 1
    
    print("\n" + "=" * 60)
    print(f"RESULTS: {passed} passed, {failed} failed")
    print("=" * 60)
    
    return failed == 0

if __name__ == "__main__":
    sys.exit(0 if main() else 1)