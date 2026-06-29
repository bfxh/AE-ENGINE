"""
LOD自动生成器 - 为GLB模型生成多级LOD
"""

import json
import os
import sys
import subprocess
from pathlib import Path

BLENDER_PATH = "E:\\SteamLibrary\\steamapps\\common\\Blender\\blender.exe"

LOD_CONFIGS = [
    {"level": 0, "max_faces": 5000, "target_ratio": 1.0, "distance": 30},
    {"level": 1, "max_faces": 2000, "target_ratio": 0.4, "distance": 80},
    {"level": 2, "max_faces": 500, "target_ratio": 0.1, "distance": 200},
]

BLENDER_LOD_SCRIPT = """
import bpy
import sys
import os

args = sys.argv[sys.argv.index("--") + 1:]
input_path = args[0]
output_dir = args[1]
target_ratios = [float(x) for x in args[2].split(",")]

bpy.ops.wm.open_mainfile(filepath=input_path)

for obj in bpy.context.scene.objects:
    if obj.type != 'MESH':
        continue

    original_count = len(obj.data.polygons)

    for i, ratio in enumerate(target_ratios):
        bpy.ops.object.select_all(action='DESELECT')
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj

        target_count = max(8, int(original_count * ratio))

        modifier = obj.modifiers.new(name=f"Decimate_LOD{i}", type='DECIMATE')
        modifier.ratio = ratio
        modifier.use_collapse_triangulate = True

        bpy.ops.object.modifier_apply(modifier=modifier.name)

        out_name = f"LOD{i}_{os.path.splitext(os.path.basename(input_path))[0]}.glb"
        out_path = os.path.join(output_dir, out_name)

        bpy.ops.export_scene.gltf(
            filepath=out_path,
            use_selection=True,
            export_format='GLB',
            export_apply_modifiers=True,
        )
        print(f"[OK] LOD{i}: {len(obj.data.polygons)} faces -> {out_path}")

        bpy.ops.object.undo()

print("[DONE] LOD generation complete")
"""


def generate_lods(input_path: str, output_dir: str = None) -> list:
    if output_dir is None:
        output_dir = str(Path(input_path).parent / "lods")
    os.makedirs(output_dir, exist_ok=True)

    ratios = ",".join(str(c["target_ratio"]) for c in LOD_CONFIGS)

    script_path = os.path.join(output_dir, "_lod_script.py")
    with open(script_path, "w", encoding="utf-8") as f:
        f.write(BLENDER_LOD_SCRIPT)

    cmd = [BLENDER_PATH, "--background", "--python", script_path,
           "--", input_path, output_dir, ratios]

    print(f"[RUN] Generating LODs for {input_path}...")
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)

    lod_files = []
    for f in sorted(Path(output_dir).glob("LOD*.glb")):
        lod_files.append(str(f))

    if os.path.exists(script_path):
        os.remove(script_path)

    print(f"[OK] Generated {len(lod_files)} LOD files")
    return lod_files


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python lod_generator.py <input.glb> [output_dir]")
        sys.exit(1)
    generate_lods(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None)
