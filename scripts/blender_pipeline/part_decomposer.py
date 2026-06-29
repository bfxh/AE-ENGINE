"""
部件分解器 - 将3D模型按部件分割为独立GLB文件
输入: 带manifest.json的模型目录
输出: 每个部件一个GLB文件
"""

import json
import os
import sys
import shutil
from pathlib import Path

BLENDER_PATH = "E:\\SteamLibrary\\steamapps\\common\\Blender\\blender.exe"

BLENDER_PART_SCRIPT = """
import bpy
import json
import os
import sys

args = sys.argv[sys.argv.index("--") + 1:]
model_path = args[0]
manifest_path = args[1]
output_dir = args[2]

with open(manifest_path, "r", encoding="utf-8") as f:
    manifest = json.load(f)

bpy.ops.wm.open_mainfile(filepath=model_path)

parts = manifest.get("parts", [])
for part in parts:
    part_name = part["name"]
    mesh_path = part.get("mesh_path", f"{part_name}.glb")

    obj_names = []
    for obj in bpy.data.objects:
        if obj.type == 'MESH':
            obj_lower = obj.name.lower().replace(" ", "_")
            part_lower = part_name.lower().replace(" ", "_")
            if part_lower in obj_lower or obj_lower in part_lower:
                obj_names.append(obj.name)

    if not obj_names:
        for obj in bpy.data.objects:
            if obj.type == 'MESH':
                obj_names.append(obj.name)
                break

    if not obj_names:
        print(f"[SKIP] No mesh found for part: {part_name}")
        continue

    bpy.ops.object.select_all(action='DESELECT')
    for name in obj_names:
        if name in bpy.data.objects:
            bpy.data.objects[name].select_set(True)

    bpy.ops.object.duplicate()
    selected = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']

    for obj in bpy.data.objects:
        if obj not in selected and obj.type == 'MESH':
            obj.select_set(False)

    bpy.ops.object.join()

    active = bpy.context.active_object
    if active:
        active.name = part_name
        active.location = (0, 0, 0)

        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)

        out_path = os.path.join(output_dir, mesh_path)
        bpy.ops.export_scene.gltf(
            filepath=out_path,
            use_selection=True,
            export_format='GLB',
            export_apply_modifiers=True,
            export_normals=True,
            export_materials='EXPORT',
            export_colors=True,
            export_cameras=False,
            export_lights=False,
        )
        print(f"[OK] Exported: {out_path}")

    bpy.ops.object.delete()

print("[DONE] All parts exported")
"""


def decompose(model_path: str, manifest_path: str, output_dir: str = None) -> list:
    if output_dir is None:
        output_dir = str(Path(model_path).parent / "parts")
    os.makedirs(output_dir, exist_ok=True)

    textures_dir = os.path.join(output_dir, "textures")
    os.makedirs(textures_dir, exist_ok=True)

    script_path = os.path.join(output_dir, "_decompose_script.py")
    with open(script_path, "w", encoding="utf-8") as f:
        f.write(BLENDER_PART_SCRIPT)

    cmd = [
        BLENDER_PATH,
        "--background",
        "--python", script_path,
        "--", model_path, manifest_path, output_dir
    ]

    print(f"[RUN] Blender part decomposition...")
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)

    exported = []
    if result.returncode == 0:
        for f in os.listdir(output_dir):
            if f.endswith(".glb") and not f.startswith("_"):
                exported.append(os.path.join(output_dir, f))
        print(f"[OK] Exported {len(exported)} part GLBs")
    else:
        print(f"[ERROR] Blender failed: {result.stderr[-500:]}")

    if os.path.exists(script_path):
        os.remove(script_path)

    return exported


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python part_decomposer.py <model.glb> <manifest.json> [output_dir]")
        sys.exit(1)
    decompose(sys.argv[1], sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else None)
