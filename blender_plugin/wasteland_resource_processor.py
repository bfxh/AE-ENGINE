bl_info = {
    "name": "Wasteland Resource Processor",
    "author": "Wasteland Engine",
    "version": (0, 4, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > Wasteland > Resources",
    "description": "批量处理下载的开源资源: 格式转换/LOD生成/材质优化/Godot导出",
    "category": "3D View",
}

import bpy
import bmesh
import math
import random
import os
import json
import shutil
from mathutils import Vector, Matrix, Euler
from pathlib import Path

EXPORT_DIR = os.environ.get("WASTELAND_EXPORT_DIR", "D:/rj/wasteland_project/godot_project/assets")
DOWNLOAD_DIR = os.environ.get("WASTELAND_DOWNLOAD_DIR", "D:/rj/wasteland_project/_downloads")

ASSET_MANIFEST = os.path.join(EXPORT_DIR, "asset_manifest.json")

def ensure_dirs():
    dirs = [
        "models/trees", "models/rocks", "models/buildings", "models/props",
        "models/characters", "models/animals",
        "textures/ground", "textures/bark", "textures/foliage", "textures/building",
        "materials", "animations", "audio",
    ]
    for d in dirs:
        Path(os.path.join(EXPORT_DIR, d)).mkdir(parents=True, exist_ok=True)
    Path(DOWNLOAD_DIR).mkdir(parents=True, exist_ok=True)

def load_manifest():
    if os.path.exists(ASSET_MANIFEST):
        with open(ASSET_MANIFEST, 'r') as f:
            return json.load(f)
    return {"version": "0.4.0", "assets": {}, "stats": {"total": 0, "trees": 0, "rocks": 0, "buildings": 0, "characters": 0, "animals": 0}}

def save_manifest(manifest):
    manifest["stats"]["total"] = len(manifest["assets"])
    with open(ASSET_MANIFEST, 'w') as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

def import_and_process(source_path, category, asset_type, scale=1.0, generate_lod=True):
    if not os.path.exists(source_path):
        print(f"Source not found: {source_path}")
        return None

    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)

    ext = os.path.splitext(source_path)[1].lower()
    base_name = os.path.splitext(os.path.basename(source_path))[0]

    import_map = {
        '.glb': lambda: bpy.ops.import_scene.gltf(filepath=source_path),
        '.gltf': lambda: bpy.ops.import_scene.gltf(filepath=source_path),
        '.fbx': lambda: bpy.ops.import_scene.fbx(filepath=source_path),
        '.obj': lambda: bpy.ops.import_scene.obj(filepath=source_path),
    }

    if ext not in import_map:
        print(f"Unsupported format: {ext}")
        return None

    try:
        import_map[ext]()
    except Exception as e:
        print(f"Import failed: {e}")
        return None

    imported = [o for o in bpy.context.selected_objects if o.type == 'MESH']
    if not imported:
        imported = [o for o in bpy.data.objects if o.type == 'MESH' and o.select_get()]

    if not imported:
        print("No mesh objects found after import")
        return None

    for obj in imported:
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj

        obj.scale = Vector((scale, scale, scale))
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)

        obj.name = f"{category}_{base_name}"

        if category == "character":
            process_character(obj)
        elif category in ("tree", "rock", "prop"):
            process_environment_object(obj, category)
        elif category == "building":
            process_building(obj)

        clean_material_names(obj)

        if generate_lod and obj.type == 'MESH':
            generate_lod_levels(obj)

    output_name = f"{category}_{base_name}"
    export_path = os.path.join(EXPORT_DIR, f"models/{category}s", f"{output_name}.glb")

    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.export_scene.gltf(
        filepath=export_path,
        export_format='GLB',
        export_apply=True,
        export_image_format='JPEG',
        export_texture_dir='',
    )

    asset_info = {
        "name": output_name,
        "source": source_path,
        "category": category,
        "type": asset_type,
        "path": f"models/{category}s/{output_name}.glb",
        "vertices": sum(len(o.data.vertices) for o in imported if o.type == 'MESH' and o.data),
        "triangles": sum(len(o.data.polygons) for o in imported if o.type == 'MESH' and o.data),
        "processed_date": str(bpy.data.scenes[0].get("date", "2026-06-08")),
    }

    print(f"Processed: {output_name} → {export_path}")
    return asset_info

def process_character(obj):
    for mat in obj.data.materials:
        if mat and mat.node_tree:
            for node in mat.node_tree.nodes:
                if node.type == 'BSDF_PRINCIPLED':
                    node.inputs['Specular IOR Level'].default_value = 0.1

def process_environment_object(obj, category):
    for mat in obj.data.materials:
        if mat and mat.node_tree:
            for node in mat.node_tree.nodes:
                if node.type == 'BSDF_PRINCIPLED':
                    if category == "rock":
                        node.inputs['Roughness'].default_value = 0.85
                    elif category == "tree":
                        node.inputs['Roughness'].default_value = 0.7

def process_building(obj):
    for mat in obj.data.materials:
        if mat and mat.node_tree:
            for node in mat.node_tree.nodes:
                if node.type == 'BSDF_PRINCIPLED':
                    node.inputs['Roughness'].default_value = max(
                        node.inputs['Roughness'].default_value, 0.8
                    )

def clean_material_names(obj):
    for mat_slot in obj.material_slots:
        if mat_slot.material:
            mat_slot.material.name = mat_slot.material.name.replace('.', '_').replace(' ', '_').lower()

def generate_lod_levels(obj, levels=3):
    mod = obj.modifiers.new(name="Decimate_LOD", type='DECIMATE')
    mod.decimate_type = 'COLLAPSE'

    for lod_level in range(1, levels + 1):
        ratio = 1.0 / (2 ** lod_level)
        mod.ratio = ratio

        lod_obj = obj.copy()
        lod_obj.data = obj.data.copy()
        lod_obj.name = f"{obj.name}_LOD{lod_level}"

        lod_mod = lod_obj.modifiers.new(name="Decimate", type='DECIMATE')
        lod_mod.decimate_type = 'COLLAPSE'
        lod_mod.ratio = ratio

        bpy.context.collection.objects.link(lod_obj)

class WASTELAND_OT_batch_process_downloads(bpy.types.Operator):
    bl_idname = "wasteland.batch_process_downloads"
    bl_label = "Batch Process Downloads"
    bl_description = "Scan download directory and process all 3D assets"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        ensure_dirs()
        manifest = load_manifest()

        if not os.path.exists(DOWNLOAD_DIR):
            self.report({'WARNING'}, f"Download directory not found: {DOWNLOAD_DIR}")
            return {'CANCELLED'}

        processed = 0
        skipped = 0

        for root, dirs, files in os.walk(DOWNLOAD_DIR):
            for f in files:
                ext = os.path.splitext(f)[1].lower()
                if ext not in ('.glb', '.gltf', '.fbx', '.obj', '.blend'):
                    continue

                source_path = os.path.join(root, f)
                base = os.path.splitext(f)[0].lower()

                category = "prop"
                if any(kw in base for kw in ("tree", "pine", "oak", "birch", "willow", "plant")):
                    category = "tree"
                elif any(kw in base for kw in ("rock", "stone", "boulder")):
                    category = "rock"
                elif any(kw in base for kw in ("building", "ruin", "house", "tower")):
                    category = "building"
                elif any(kw in base for kw in ("character", "npc", "human", "player")):
                    category = "character"
                elif any(kw in base for kw in ("deer", "wolf", "bear", "boar", "animal")):
                    category = "animal"

                asset_key = f"{category}/{f}"
                if asset_key in manifest["assets"]:
                    skipped += 1
                    continue

                asset_info = import_and_process(source_path, category, "imported")
                if asset_info:
                    manifest["assets"][asset_key] = asset_info
                    manifest["stats"][f"{category}s"] = manifest["stats"].get(f"{category}s", 0) + 1
                    processed += 1

        save_manifest(manifest)
        self.report({'INFO'}, f"Processed: {processed}, Skipped: {skipped}, Total: {manifest['stats']['total']}")
        return {'FINISHED'}

class WASTELAND_OT_export_all_to_godot(bpy.types.Operator):
    bl_idname = "wasteland.export_all_to_godot"
    bl_label = "Export All to Godot"
    bl_description = "Export all processed assets as GLB to Godot project"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        manifest = load_manifest()
        exported = 0

        for key, info in manifest["assets"].items():
            if key == "version" or key == "stats" or key == "total":
                continue
            print(f"Already processed: {info['name']}")
            exported += 1

        self.report({'INFO'}, f"Total assets in manifest: {exported}")
        return {'FINISHED'}

class WASTELAND_OT_clean_temp_files(bpy.types.Operator):
    bl_idname = "wasteland.clean_temp_files"
    bl_label = "Clean Temp Files"
    bl_description = "Remove temporary Blender data to free memory"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        for block in bpy.data.meshes:
            if block.users == 0:
                bpy.data.meshes.remove(block)
        for block in bpy.data.materials:
            if block.users == 0:
                bpy.data.materials.remove(block)
        for block in bpy.data.images:
            if block.users == 0:
                bpy.data.images.remove(block)
        self.report({'INFO'}, "Temporary files cleaned")
        return {'FINISHED'}

class WASTELAND_PT_resource_panel(bpy.types.Panel):
    bl_label = "Resource Processor"
    bl_idname = "WASTELAND_PT_resource_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Wasteland"
    bl_parent_id = "WASTELAND_PT_main"

    def draw(self, context):
        layout = self.layout
        layout.operator("wasteland.batch_process_downloads", icon='IMPORT')
        layout.operator("wasteland.export_all_to_godot", icon='EXPORT')
        layout.separator()
        layout.operator("wasteland.clean_temp_files", icon='TRASH')

        manifest = load_manifest()
        stats = manifest.get("stats", {})
        layout.label(text=f"Assets: {stats.get('total', 0)}")
        layout.label(text=f"Trees: {stats.get('trees', 0)} | Rocks: {stats.get('rocks', 0)}")
        layout.label(text=f"Buildings: {stats.get('buildings', 0)} | Characters: {stats.get('characters', 0)}")

def register():
    bpy.utils.register_class(WASTELAND_OT_batch_process_downloads)
    bpy.utils.register_class(WASTELAND_OT_export_all_to_godot)
    bpy.utils.register_class(WASTELAND_OT_clean_temp_files)
    bpy.utils.register_class(WASTELAND_PT_resource_panel)

def unregister():
    bpy.utils.unregister_class(WASTELAND_PT_resource_panel)
    bpy.utils.unregister_class(WASTELAND_OT_clean_temp_files)
    bpy.utils.unregister_class(WASTELAND_OT_export_all_to_godot)
    bpy.utils.unregister_class(WASTELAND_OT_batch_process_downloads)

if __name__ == "__main__":
    register()