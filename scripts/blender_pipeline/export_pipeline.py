"""
导出管线 - 批量导出GLB + Schema到Godot项目
"""

import json
import os
import shutil
import sys
from pathlib import Path

GODOT_ASSETS_DIR = "d:\\rj\\wasteland_project\\godot_project\\assets"


def export_to_godot(asset_dir: str, category: str = "misc") -> str:
    target_dir = os.path.join(GODOT_ASSETS_DIR, category)
    os.makedirs(target_dir, exist_ok=True)

    src = Path(asset_dir)

    manifest_src = src / "manifest.json"
    if manifest_src.exists():
        shutil.copy2(str(manifest_src), os.path.join(target_dir, "manifest.json"))

    for glb in src.glob("*.glb"):
        shutil.copy2(str(glb), os.path.join(target_dir, glb.name))

    lods_dir = src / "lods"
    if lods_dir.exists():
        target_lods = os.path.join(target_dir, "lods")
        os.makedirs(target_lods, exist_ok=True)
        for lod in lods_dir.glob("*.glb"):
            shutil.copy2(str(lod), os.path.join(target_lods, lod.name))

    parts_dir = src / "parts"
    if parts_dir.exists():
        target_parts = os.path.join(target_dir, "parts")
        os.makedirs(target_parts, exist_ok=True)
        for part in parts_dir.glob("*.glb"):
            shutil.copy2(str(part), os.path.join(target_parts, part.name))

    textures_dir = src / "textures"
    if textures_dir.exists():
        target_textures = os.path.join(target_dir, "textures")
        os.makedirs(target_textures, exist_ok=True)
        for tex in textures_dir.glob("*"):
            if tex.is_file():
                shutil.copy2(str(tex), os.path.join(target_textures, tex.name))

    preview = src / "preview.png"
    if preview.exists():
        shutil.copy2(str(preview), os.path.join(target_dir, "preview.png"))

    print(f"[OK] Exported to {target_dir}")
    return target_dir


def register_godot_import(target_dir: str) -> None:
    manifest_path = os.path.join(target_dir, "manifest.json")
    if not os.path.exists(manifest_path):
        return

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    for part in manifest.get("parts", []):
        mesh_path = part.get("mesh_path", "")
        if mesh_path:
            glb_path = os.path.join(target_dir, mesh_path)
            if os.path.exists(glb_path):
                import_file = glb_path + ".import"
                if not os.path.exists(import_file):
                    with open(import_file, "w") as f:
                        f.write("[remap]\nimporter=\"texture\"\n")

    print(f"[OK] Godot import hints registered")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python export_pipeline.py <asset_dir> [category]")
        sys.exit(1)
    category = sys.argv[2] if len(sys.argv) > 2 else "misc"
    target = export_to_godot(sys.argv[1], category)
    register_godot_import(target)
