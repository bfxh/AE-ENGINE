#!/usr/bin/env python3
"""
Wasteland 资源自动下载器
自动从网络下载 CC0 开源资源到项目目录
"""

import os
import sys
import json
import urllib.request
import urllib.error
import zipfile
import shutil
import hashlib
from pathlib import Path
from datetime import datetime

IS_WINDOWS = os.name == 'nt'
HOME = Path.home()
PROJECT_ROOT = Path("d:/rj/wasteland_project") if IS_WINDOWS else Path.home() / "wasteland_project"
ASSETS_DIR = PROJECT_ROOT / "godot_project" / "assets"
DOWNLOADS_DIR = PROJECT_ROOT / "_downloads"
MANIFEST_FILE = ASSETS_DIR / "asset_manifest.json"

RESOURCES = {
    "characters": [
        {
            "name": "quaternius_base_characters",
            "url": "https://quaternius.com/packs/universalbasecharacters.html",
            "note": "手动下载: 选GLB格式, 放入 assets/models/characters/",
            "manual": True,
            "destination": "models/characters/",
        },
        {
            "name": "quaternius_animation_library",
            "url": "https://quaternius.itch.io/universal-animation-library",
            "note": "手动下载: itch.io, 选Godot GLB格式",
            "manual": True,
            "destination": "animations/",
        },
        {
            "name": "quaternius_animation_library_2",
            "url": "https://quaternius.itch.io/universal-animation-library-2",
            "note": "手动下载: 130+ extra animations",
            "manual": True,
            "destination": "animations/",
        },
    ],
    "environment": [
        {
            "name": "quaternius_stylized_nature",
            "url": "https://quaternius.com/packs/ultimatestylizednature.html",
            "note": "手动下载: 树木+植被, CC0",
            "manual": True,
            "destination": "models/trees/",
        },
        {
            "name": "quaternius_ultimate_nature",
            "url": "https://quaternius.com/packs/ultimatenaturepack.html",
            "note": "手动下载: Ult Nature Pack, CC0",
            "manual": True,
            "destination": "models/trees/",
        },
        {
            "name": "quaternius_animated_animals",
            "url": "https://quaternius.com/packs/ultimateanimatedanimalpack.html",
            "note": "手动下载: 动物模型+动画, CC0",
            "manual": True,
            "destination": "models/animals/",
        },
    ],
    "buildings_ruins": [
        {
            "name": "quaternius_ruins_pack",
            "url": "https://quaternius.com/packs/ultimatemodularruinspack.html",
            "note": "手动下载: 废墟组件, CC0",
            "manual": True,
            "destination": "models/buildings/",
        },
        {
            "name": "quaternius_zombie_kit",
            "url": "https://quaternius.com/packs/zombieapocalypsekit.html",
            "note": "手动下载: 僵尸末日场景包, CC0",
            "manual": True,
            "destination": "models/buildings/",
        },
    ],
    "textures": [
        {
            "name": "polyhaven_forest_floor",
            "url": "https://polyhaven.com/a/forest_floor",
            "note": "手动下载: 选2K JPG, 放入 textures/ground/",
            "manual": True,
            "destination": "textures/ground/",
        },
        {
            "name": "polyhaven_forest_leaves_02",
            "url": "https://polyhaven.com/a/forest_leaves_02",
            "note": "手动下载: 森林落叶地面",
            "manual": True,
            "destination": "textures/ground/",
        },
        {
            "name": "polyhaven_forest_ground_01",
            "url": "https://polyhaven.com/a/forrest_ground_01",
            "note": "手动下载: 森林地面+草+树枝",
            "manual": True,
            "destination": "textures/ground/",
        },
    ],
    "hdri": [
        {
            "name": "polyhaven_hdri_forest",
            "url": "https://polyhaven.com/hdris?categories=outdoor",
            "note": "手动下载: 选2K HDR, 放入 hdri/",
            "manual": True,
            "destination": "hdri/",
        },
    ],
}

def ensure_dirs():
    dirs = [
        "models/trees", "models/rocks", "models/buildings", "models/props",
        "models/characters", "models/animals",
        "textures/ground", "textures/bark", "textures/foliage", "textures/building",
        "materials", "animations", "audio", "hdri",
    ]
    for d in dirs:
        (ASSETS_DIR / d).mkdir(parents=True, exist_ok=True)
    DOWNLOADS_DIR.mkdir(parents=True, exist_ok=True)

def load_manifest():
    if MANIFEST_FILE.exists():
        with open(MANIFEST_FILE, 'r') as f:
            return json.load(f)
    return {
        "version": "0.4.0",
        "created": datetime.now().isoformat(),
        "assets": {},
    }

def save_manifest(manifest):
    manifest["updated"] = datetime.now().isoformat()
    with open(MANIFEST_FILE, 'w') as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

def download_file(url, dest_path):
    print(f"  Downloading: {url}")
    try:
        req = urllib.request.Request(url, headers={
            'User-Agent': 'WastelandAssetDownloader/1.0'
        })
        with urllib.request.urlopen(req, timeout=60) as response:
            with open(dest_path, 'wb') as f:
                f.write(response.read())
        return True
    except Exception as e:
        print(f"  Download failed: {e}")
        return False

def extract_zip(zip_path, extract_dir):
    print(f"  Extracting: {zip_path} -> {extract_dir}")
    try:
        with zipfile.ZipFile(zip_path, 'r') as zf:
            zf.extractall(extract_dir)
        return True
    except Exception as e:
        print(f"  Extract failed: {e}")
        return False

def compute_hash(file_path):
    sha256 = hashlib.sha256()
    with open(file_path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            sha256.update(chunk)
    return sha256.hexdigest()

def scan_downloads():
    print("\n" + "=" * 60)
    print("SCANNING DOWNLOADS DIRECTORY")
    print("=" * 60)

    manifest = load_manifest()
    found = 0

    for root, dirs, files in os.walk(DOWNLOADS_DIR):
        for f in files:
            ext = os.path.splitext(f)[1].lower()
            if ext not in ('.zip', '.glb', '.gltf', '.fbx', '.blend', '.png', '.jpg', '.hdr', '.exr'):
                continue

            file_path = os.path.join(root, f)
            rel_path = os.path.relpath(file_path, DOWNLOADS_DIR)
            file_hash = compute_hash(file_path)

            if rel_path not in manifest["assets"]:
                manifest["assets"][rel_path] = {
                    "filename": f,
                    "path": rel_path,
                    "hash": file_hash,
                    "size": os.path.getsize(file_path),
                    "status": "downloaded",
                    "date": datetime.now().isoformat(),
                }
                found += 1
                print(f"  NEW: {rel_path} ({manifest['assets'][rel_path]['size'] // 1024} KB)")

    save_manifest(manifest)
    print(f"\nTotal tracked: {len(manifest['assets'])} assets (new: {found})")
    return manifest

def process_zip_to_assets(manifest):
    print("\n" + "=" * 60)
    print("PROCESSING ZIP -> ASSETS")
    print("=" * 60)

    processed = 0
    for rel_path, info in manifest["assets"].items():
        if info.get("status") == "processed":
            continue
        if not rel_path.endswith('.zip'):
            continue

        zip_path = DOWNLOADS_DIR / rel_path
        extract_dir = DOWNLOADS_DIR / os.path.splitext(rel_path)[0]

        if extract_zip(str(zip_path), str(extract_dir)):
            for root, dirs, files in os.walk(extract_dir):
                for f in files:
                    ext = os.path.splitext(f)[1].lower()
                    if ext in ('.glb', '.gltf', '.fbx', '.blend', '.png', '.jpg'):
                        src = os.path.join(root, f)

                        category = "models/props/"
                        base = f.lower()
                        if any(k in base for k in ("tree", "plant", "pine", "oak")):
                            category = "models/trees/"
                        elif any(k in base for k in ("character", "npc", "human")):
                            category = "models/characters/"
                        elif any(k in base for k in ("animal", "deer", "wolf")):
                            category = "models/animals/"
                        elif any(k in base for k in ("building", "ruin", "house")):
                            category = "models/buildings/"
                        elif any(k in base for k in ("rock", "stone")):
                            category = "models/rocks/"
                        elif ext in ('.png', '.jpg'):
                            category = "textures/"

                        dest = ASSETS_DIR / category / f
                        dest.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copy2(src, dest)
                        processed += 1
                        print(f"  COPIED: {f} -> {category}")

            info["status"] = "processed"

    save_manifest(manifest)
    print(f"\nTotal files copied: {processed}")

def print_manual_instructions():
    print("\n" + "=" * 60)
    print("MANUAL DOWNLOAD INSTRUCTIONS")
    print("=" * 60)

    categories = {}

    for cat, items in RESOURCES.items():
        for item in items:
            if item.get("manual", False):
                if cat not in categories:
                    categories[cat] = []
                categories[cat].append(item)

    for cat, items in categories.items():
        print(f"\n[{cat.upper()}]")
        for item in items:
            dest_path = ASSETS_DIR / item["destination"]
            dest_path.mkdir(parents=True, exist_ok=True)
            print(f"  {item['name']}:")
            print(f"    URL: {item['url']}")
            print(f"    -> {dest_path}")
            print(f"    Note: {item['note']}")
            print()

    print("=" * 60)
    print("ALL URLs for quick access:")
    for cat, items in RESOURCES.items():
        for item in items:
            if item.get("manual", False):
                print(f"  {item['url']}")

def check_godot_available():
    godot_paths = [
        "godot", "godot.exe",
        "D:/godot/Godot_v4.6-stable_win64.exe",
        "C:/Program Files/Godot/Godot_v4.6.exe",
    ]
    for p in godot_paths:
        try:
            result = os.popen(f'"{p}" --version 2>&1').read()
            if "Godot" in result or "4." in result:
                print(f"\nGodot found: {p}")
                return p
        except:
            pass
    return None

def main():
    print("=" * 60)
    print("WASTELAND RESOURCE DOWNLOADER")
    print("=" * 60)

    ensure_dirs()

    if len(sys.argv) > 1 and sys.argv[1] == "--instructions":
        print_manual_instructions()
        return

    scan_downloads()

    godot_path = check_godot_available()
    if godot_path:
        print(f"\nGodot available: {godot_path}")
        print("Run Godot validation:")
        print(f'  "{godot_path}" --headless --path "{PROJECT_ROOT / "godot_project"}" --check-only --quit 2>&1')
    else:
        print("\nGodot not found in PATH. Install Godot 4.6+ first.")
        print("Download: https://godotengine.org/download/windows/")

    print_manual_instructions()

    print("\nDONE. Next steps:")
    print("1. Visit the URLs above and download GLB format assets")
    print("2. Place downloaded files in:", DOWNLOADS_DIR)
    print("3. Run this script again to track them")
    print(f"4. Open Blender and run wasteland_resource_processor.py")
    print(f"5. Open Godot project: {PROJECT_ROOT / 'godot_project'}")

if __name__ == "__main__":
    main()