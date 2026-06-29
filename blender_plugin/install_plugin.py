"""
Wasteland Blender Plugin Auto-Installer
Automatically installs the Wasteland Generator addon to Blender.
"""
import os
import sys
import shutil
import json
import subprocess

def find_blender_installations():
    """Find all Blender installations on the system."""
    paths = []
    
    # Common installation locations
    candidates = [
        r"C:\Program Files\Blender Foundation\Blender",
        r"C:\Program Files (x86)\Blender Foundation\Blender",
        r"E:\SteamLibrary\steamapps\common\Blender",
        r"D:\SteamLibrary\steamapps\common\Blender",
        r"C:\SteamLibrary\steamapps\common\Blender",
        os.path.expanduser(r"~\AppData\Roaming\Blender Foundation\Blender"),
        os.path.expanduser(r"~\Documents\Blender"),
        r"E:\C盘迁移文件\游戏\建模与游戏引擎\New Folder",
    ]
    
    for base in candidates:
        if os.path.exists(base):
            blender_exe = os.path.join(base, "blender.exe")
            if os.path.exists(blender_exe):
                # Extract version from path or directory name
                version = os.path.basename(base)
                if version.lower() == "blender":
                    version = "5.x"
                paths.append({
                    "path": base,
                    "version": version,
                    "executable": blender_exe
                })
            else:
                # Check subdirectories for versioned installations
                for item in os.listdir(base):
                    item_path = os.path.join(base, item)
                    if os.path.isdir(item_path):
                        blender_exe = os.path.join(item_path, "blender.exe")
                        if os.path.exists(blender_exe):
                            paths.append({
                                "path": item_path,
                                "version": item,
                                "executable": blender_exe
                            })
    
    return paths

def get_addons_path(blender_path, version):
    """Get the addons path for a specific Blender version."""
    return os.path.join(blender_path, version, "scripts", "addons")

def install_plugin(source_path, addons_path, plugin_name="wasteland_generator"):
    """Install the plugin to Blender's addons directory."""
    plugin_dest = os.path.join(addons_path, plugin_name)
    
    # Remove existing installation
    if os.path.exists(plugin_dest):
        shutil.rmtree(plugin_dest)
        print(f"Removed existing installation: {plugin_dest}")
    
    # Create plugin directory
    os.makedirs(plugin_dest, exist_ok=True)
    
    # Copy all files from source
    for item in os.listdir(source_path):
        src_item = os.path.join(source_path, item)
        dest_item = os.path.join(plugin_dest, item)
        
        if os.path.isfile(src_item):
            shutil.copy2(src_item, dest_item)
            print(f"Copied: {item}")
        elif os.path.isdir(src_item):
            shutil.copytree(src_item, dest_item)
            print(f"Copied directory: {item}")
    
    print(f"\nPlugin installed to: {plugin_dest}")
    return True

def enable_plugin(blender_exe, plugin_name="wasteland_generator"):
    """Enable the plugin in Blender's preferences."""
    script = f"""
import bpy
import addon_utils

# Try to enable the addon
addon_utils.enable("{plugin_name}", default_set=True, persistent=True)

# Save user preferences
bpy.ops.wm.save_userpref()

print(f"Addon {{plugin_name}} enabled successfully")
"""
    
    # Write temporary script
    import tempfile
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(script)
        temp_script = f.name
    
    try:
        result = subprocess.run(
            [blender_exe, "--background", "--python", temp_script],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        if result.returncode == 0:
            print("Plugin enabled successfully")
            return True
        else:
            print(f"Enable failed: {result.stderr}")
            return False
    finally:
        os.unlink(temp_script)

def create_installation_report(report_path, installations, installed_to):
    """Create an installation report."""
    report = {
        "timestamp": __import__('datetime').datetime.now().isoformat(),
        "blender_installations": installations,
        "installed_to": installed_to,
        "plugin_files": [f for f in os.listdir(PLUGIN_SOURCE) if f.endswith('.py')],
        "status": "SUCCESS"
    }
    
    with open(report_path, 'w') as f:
        json.dump(report, f, indent=2)
    
    print(f"\nInstallation report saved to: {report_path}")

PLUGIN_SOURCE = os.path.dirname(os.path.abspath(__file__))

def main():
    print("=" * 60)
    print("WASTELAND BLENDER PLUGIN AUTO-INSTALLER")
    print("=" * 60)
    
    # Find Blender installations
    print("\n[1/4] Searching for Blender installations...")
    installations = find_blender_installations()
    
    if not installations:
        print("ERROR: No Blender installations found!")
        sys.exit(1)
    
    print(f"Found {len(installations)} Blender installation(s):")
    for i, inst in enumerate(installations, 1):
        print(f"  {i}. v{inst['version']} - {inst['path']}")
    
    # Select installation (use first one by default)
    selected = installations[0]
    print(f"\nSelected: Blender v{selected['version']}")
    
    # Get addons path
    addons_path = get_addons_path(selected['path'], selected['version'])
    print(f"Addons directory: {addons_path}")
    
    # Install plugin
    print("\n[2/4] Installing plugin files...")
    install_plugin(PLUGIN_SOURCE, addons_path)
    
    # Enable plugin
    print("\n[3/4] Enabling plugin in Blender...")
    enable_plugin(selected['executable'])
    
    # Create report
    print("\n[4/4] Generating installation report...")
    report_path = os.path.join(PLUGIN_SOURCE, "install_report.json")
    create_installation_report(report_path, installations, addons_path)
    
    print("\n" + "=" * 60)
    print("INSTALLATION COMPLETE")
    print("=" * 60)
    print(f"\nPlugin: Wasteland Generator")
    print(f"Location: {addons_path}\\wasteland_generator")
    print(f"Blender Version: {selected['version']}")
    print(f"\nHow to use:")
    print("1. Open Blender")
    print("2. Go to Edit > Preferences > Add-ons")
    print("3. Search for 'Wasteland Generator'")
    print("4. Enable the addon")
    print("5. Open the 3D View > Sidebar > Wasteland tab")
    print("6. Click 'Generate All'")
    print("=" * 60)

if __name__ == "__main__":
    main()