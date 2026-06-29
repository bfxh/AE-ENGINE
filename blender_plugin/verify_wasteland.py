"""
Wasteland headless generation & verification script.
Run: blender --background --python verify_wasteland.py
"""
import bpy
import sys
import os
import json
import math
import random
import time

T0 = time.time()

sys.path.insert(0, os.path.dirname(__file__))
from wasteland_generator import (
    generate_terrain, generate_buildings, generate_trees,
    WastelandProperties
)

class DummyProps:
    terrain_size = 500.0
    terrain_resolution = 128
    height_scale = 30.0
    noise_seed = 42
    building_count = 30
    building_radius = 220.0
    tree_count = 100
    tree_radius = 230.0
    export_path = "//wasteland_verify.glb"

props = DummyProps()

print("=" * 60)
print("WASTELAND VERIFICATION PIPELINE")
print("=" * 60)

print("\n[1/4] Generating terrain...")
terrain = generate_terrain(props)
terrain_verts = len(terrain.data.vertices)
terrain_faces = len(terrain.data.polygons)
print(f"  Terrain: {terrain_verts} verts, {terrain_faces} faces")

print("\n[2/4] Generating buildings...")
buildings_before = len([o for o in bpy.data.objects if o.name.startswith("Wasteland_Building")])
generate_buildings(props)
buildings_after = len([o for o in bpy.data.objects if o.name.startswith("Wasteland_Building")])
print(f"  Buildings: {buildings_after - buildings_before} generated")

print("\n[3/4] Generating trees...")
trees_before = len([o for o in bpy.data.objects if o.name.startswith("Wasteland_Tree")])
generate_trees(props)
trees_after = len([o for o in bpy.data.objects if o.name.startswith("Wasteland_Tree")])
print(f"  Trees: {trees_after - trees_before} generated")

print("\n[4/4] Exporting to glTF...")
export_path = os.path.abspath(bpy.path.abspath("//wasteland_verify.glb"))
bpy.ops.export_scene.gltf(
    filepath=export_path,
    export_format='GLB',
    export_apply=True,
    export_image_format='NONE',
    export_texcoords=True,
    export_normals=True,
)

total_objects = len(bpy.data.objects)
total_verts = sum(len(o.data.vertices) for o in bpy.data.objects if o.type == 'MESH')
total_faces = sum(len(o.data.polygons) for o in bpy.data.objects if o.type == 'MESH')

elapsed = time.time() - T0

result = {
    "status": "OK",
    "elapsed_seconds": round(elapsed, 2),
    "objects": total_objects,
    "total_vertices": total_verts,
    "total_faces": total_faces,
    "terrain_vertices": terrain_verts,
    "terrain_faces": terrain_faces,
    "buildings": buildings_after,
    "trees": trees_after,
    "export_path": export_path,
    "export_size_bytes": os.path.getsize(export_path) if os.path.exists(export_path) else 0,
}

print(f"\n{'=' * 60}")
print(f"VERIFICATION COMPLETE ({elapsed:.1f}s)")
print(f"{'=' * 60}")
print(f"  Objects:     {total_objects}")
print(f"  Vertices:    {total_verts:,}")
print(f"  Faces:       {total_faces:,}")
print(f"  Terrain:     {terrain_verts} verts / {terrain_faces} faces")
print(f"  Buildings:   {buildings_after}")
print(f"  Trees:       {trees_after}")
print(f"  Export:      {export_path}")
print(f"  File size:   {result['export_size_bytes']:,} bytes")
print(f"{'=' * 60}")

result_path = os.path.join(os.path.dirname(__file__), "verify_result.json")
with open(result_path, 'w') as f:
    json.dump(result, f, indent=2)
print(f"\nResult saved to: {result_path}")

bpy.ops.wm.quit_blender()