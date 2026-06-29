bl_info = {
    "name": "Wasteland Asset Pipeline",
    "author": "Wasteland Engine",
    "version": (0, 3, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > Wasteland",
    "description": "Asset pipeline for Wasteland Engine - auto-generate, process, export to Godot",
    "category": "3D View",
}

import bpy
import bmesh
import math
import random
import os
import json
from mathutils import Vector, Matrix, Euler
from pathlib import Path

EXPORT_DIR = os.environ.get("WASTELAND_EXPORT_DIR", "D:/rj/wasteland_project/godot_project/assets")
GODOT_SCALE = 1.0

def ensure_export_dir():
    Path(EXPORT_DIR).mkdir(parents=True, exist_ok=True)
    Path(os.path.join(EXPORT_DIR, "models")).mkdir(exist_ok=True)
    Path(os.path.join(EXPORT_DIR, "textures")).mkdir(exist_ok=True)
    Path(os.path.join(EXPORT_DIR, "materials")).mkdir(exist_ok=True)
    Path(os.path.join(EXPORT_DIR, "animations")).mkdir(exist_ok=True)

class WASTELAND_OT_generate_forest_assets(bpy.types.Operator):
    bl_idname = "wasteland.generate_forest_assets"
    bl_label = "Generate Forest Assets"
    bl_description = "Generate tree trunks, branches, rocks, and ground cover models"
    bl_options = {'REGISTER', 'UNDO'}

    tree_count: bpy.props.IntProperty(name="Tree Variants", default=6, min=1, max=20)
    rock_count: bpy.props.IntProperty(name="Rock Variants", default=5, min=1, max=15)
    export_format: bpy.props.EnumProperty(
        name="Export Format",
        items=[('gltf', 'glTF 2.0', 'Best for Godot 4'), ('obj', 'Wavefront OBJ', 'Universal format')],
        default='gltf'
    )

    def execute(self, context):
        ensure_export_dir()
        random.seed(42)
        self.generate_tree_trunks()
        self.generate_tree_canopies()
        self.generate_rocks()
        self.generate_stumps()
        self.report({'INFO'}, f"Generated {self.tree_count} trees + {self.rock_count} rocks")
        return {'FINISHED'}

    def generate_tree_trunks(self):
        for i in range(self.tree_count):
            bpy.ops.object.select_all(action='DESELECT')

            h = random.uniform(5, 15)
            r = random.uniform(0.2, 0.7)
            bpy.ops.mesh.primitive_cylinder_add(
                vertices=16, radius=r, depth=h,
                location=(i * 10 - 25, 0, h / 2)
            )
            trunk = bpy.context.active_object
            trunk.name = f"TreeTrunk_{i}"
            trunk.scale = (r * 0.6 / r, 1.0, r * 0.6 / r)

            bpy.ops.object.mode_set(mode='EDIT')
            bm = bmesh.from_edit_mesh(trunk.data)
            for v in bm.verts:
                if v.co.z > h * 0.6:
                    v.co.x += random.uniform(-0.15, 0.15)
                    v.co.y += random.uniform(-0.15, 0.15)
            bmesh.update_edit_mesh(trunk.data)
            bpy.ops.object.mode_set(mode='OBJECT')

            self._export_object(trunk, f"trunk_{i}")

    def generate_tree_canopies(self):
        for i in range(self.tree_count):
            bpy.ops.object.select_all(action='DESELECT')

            r = random.uniform(2.0, 4.0)
            bpy.ops.mesh.primitive_ico_sphere_add(
                subdivisions=3, radius=r,
                location=(i * 10 - 25, 8, 8)
            )
            canopy = bpy.context.active_object
            canopy.name = f"TreeCanopy_{i}"
            canopy.scale = (1.0, random.uniform(0.6, 0.9), 1.0)

            bpy.ops.object.mode_set(mode='EDIT')
            bm = bmesh.from_edit_mesh(canopy.data)
            for v in bm.verts:
                v.co += Vector((
                    random.uniform(-0.3, 0.3),
                    random.uniform(-0.3, 0.3),
                    random.uniform(-0.3, 0.3)
                )) * 0.5
            bmesh.update_edit_mesh(canopy.data)
            bpy.ops.object.mode_set(mode='OBJECT')

            self._export_object(canopy, f"canopy_{i}")

    def generate_rocks(self):
        for i in range(self.rock_count):
            bpy.ops.object.select_all(action='DESELECT')
            r = random.uniform(0.5, 2.0)
            bpy.ops.mesh.primitive_ico_sphere_add(
                subdivisions=2, radius=r,
                location=(i * 5 - 10, -15, r * 0.5)
            )
            rock = bpy.context.active_object
            rock.name = f"Rock_{i}"
            rock.scale = (
                random.uniform(0.8, 1.2),
                random.uniform(0.4, 0.8),
                random.uniform(0.7, 1.1)
            )

            bpy.ops.object.mode_set(mode='EDIT')
            bm = bmesh.from_edit_mesh(rock.data)
            for v in bm.verts:
                v.co += Vector((
                    random.uniform(-0.4, 0.4),
                    random.uniform(-0.3, 0.3),
                    random.uniform(-0.3, 0.3)
                )) * 0.4
            bmesh.update_edit_mesh(rock.data)
            bpy.ops.object.mode_set(mode='OBJECT')

            self._export_object(rock, f"rock_{i}")

    def generate_stumps(self):
        for i in range(3):
            bpy.ops.object.select_all(action='DESELECT')
            h = random.uniform(1, 3)
            r = random.uniform(0.3, 0.6)
            bpy.ops.mesh.primitive_cylinder_add(
                vertices=12, radius=r, depth=h,
                location=(i * 8 - 8, -25, h / 2)
            )
            stump = bpy.context.active_object
            stump.name = f"Stump_{i}"

            bpy.ops.object.mode_set(mode='EDIT')
            bm = bmesh.from_edit_mesh(stump.data)
            for v in bm.verts:
                if v.co.z > h * 0.4:
                    v.co.x += random.uniform(-0.2, 0.2)
                    v.co.y += random.uniform(-0.2, 0.2)
            bmesh.update_edit_mesh(stump.data)
            bpy.ops.object.mode_set(mode='OBJECT')

            self._export_object(stump, f"stump_{i}")

    def _export_object(self, obj, name):
        bpy.ops.object.select_all(action='DESELECT')
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj

        if self.export_format == 'gltf':
            path = os.path.join(EXPORT_DIR, "models", f"{name}.glb")
            bpy.ops.export_scene.gltf(
                filepath=path, use_selection=True,
                export_format='GLB', export_apply=True
            )
        else:
            path = os.path.join(EXPORT_DIR, "models", f"{name}.obj")
            bpy.ops.export_scene.obj(
                filepath=path, use_selection=True,
                use_materials=False, global_scale=GODOT_SCALE
            )

class WASTELAND_OT_generate_building_kit(bpy.types.Operator):
    bl_idname = "wasteland.generate_building_kit"
    bl_label = "Generate Building Kit"
    bl_description = "Create modular building pieces (walls, floors, roofs, doors)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        ensure_export_dir()
        self.create_wall_segment()
        self.create_floor_panel()
        self.create_roof_piece()
        self.create_doorway()
        self.create_window_frame()
        self.create_pillar()
        self.report({'INFO'}, "Building kit generated")
        return {'FINISHED'}

    def create_wall_segment(self):
        for i in range(3):
            w = [2.0, 4.0, 6.0][i]
            h = 3.0
            bpy.ops.mesh.primitive_cube_add(
                size=1.0, location=(i * 3 - 3, 20, h / 2)
            )
            wall = bpy.context.active_object
            wall.name = f"Wall_{i}"
            wall.dimensions = (w, 0.3, h)
            self._export(wall, f"wall_{i}")

    def create_floor_panel(self):
        bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 25, 0))
        floor = bpy.context.active_object
        floor.name = "Floor"
        floor.dimensions = (4.0, 4.0, 0.2)
        self._export(floor, "floor")

    def create_roof_piece(self):
        bpy.ops.mesh.primitive_cube_add(size=1.0, location=(5, 25, 3))
        roof = bpy.context.active_object
        roof.name = "Roof"
        roof.dimensions = (5.0, 5.0, 0.2)
        roof.rotation_euler = (0, 0, 0)

        for i in range(4):
            bpy.ops.mesh.primitive_cube_add(size=1.0, location=(5 + i * 1.5 - 2.25, 27, 4.5))
            tile = bpy.context.active_object
            tile.name = f"RoofTile_{i}"
            tile.dimensions = (1.2, 3.0, 0.1)
            tile.rotation_euler = (0.3, 0, 0)

        self._export(roof, "roof")

    def create_doorway(self):
        bpy.ops.mesh.primitive_cube_add(size=1.0, location=(-5, 20, 1.5))
        door = bpy.context.active_object
        door.name = "Doorway"
        door.dimensions = (1.2, 0.3, 2.4)
        self._export(door, "doorway")

    def create_window_frame(self):
        bpy.ops.mesh.primitive_cube_add(size=1.0, location=(-8, 20, 2))
        win = bpy.context.active_object
        win.name = "Window"
        win.dimensions = (1.0, 0.15, 1.2)
        self._export(win, "window")

    def create_pillar(self):
        bpy.ops.mesh.primitive_cylinder_add(
            vertices=8, radius=0.15, depth=4.0,
            location=(5, 20, 2)
        )
        pillar = bpy.context.active_object
        pillar.name = "Pillar"
        self._export(pillar, "pillar")

    def _export(self, obj, name):
        bpy.ops.object.select_all(action='DESELECT')
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        path = os.path.join(EXPORT_DIR, "models", f"building_{name}.glb")
        bpy.ops.export_scene.gltf(filepath=path, use_selection=True, export_format='GLB')

class WASTELAND_OT_import_and_cleanup(bpy.types.Operator):
    bl_idname = "wasteland.import_and_cleanup"
    bl_label = "Import & Optimize"
    bl_description = "Import external model, decimate, generate LOD, export for Godot"
    bl_options = {'REGISTER', 'UNDO'}

    filepath: bpy.props.StringProperty(subtype='FILE_PATH')
    lod_levels: bpy.props.IntProperty(name="LOD Levels", default=3, min=1, max=5)
    decimate_ratio: bpy.props.FloatProperty(name="Decimate Ratio", default=0.5, min=0.1, max=1.0)

    def execute(self, context):
        ensure_export_dir()
        if not self.filepath:
            self.report({'ERROR'}, "No file selected")
            return {'CANCELLED'}

        name = os.path.splitext(os.path.basename(self.filepath))[0]
        ext = os.path.splitext(self.filepath)[1].lower()

        if ext in ('.obj', '.fbx', '.gltf', '.glb', '.dae', '.3ds', '.ply', '.stl'):
            if ext in ('.obj',):
                bpy.ops.wm.obj_import(filepath=self.filepath)
            elif ext in ('.fbx',):
                bpy.ops.import_scene.fbx(filepath=self.filepath)
            elif ext in ('.gltf', '.glb'):
                bpy.ops.import_scene.gltf(filepath=self.filepath)
            elif ext in ('.dae',):
                bpy.ops.wm.collada_import(filepath=self.filepath)
            else:
                self.report({'ERROR'}, f"Unsupported format: {ext}")
                return {'CANCELLED'}

        imported = [o for o in bpy.context.selected_objects]
        if not imported:
            imported = [o for o in bpy.data.objects if o.name.startswith(name)]
        if not imported:
            self.report({'ERROR'}, "No objects imported")
            return {'CANCELLED'}

        for obj in imported:
            obj.select_set(True)
            bpy.context.view_layer.objects.active = obj
            self._optimize_object(obj)

        if imported:
            bpy.ops.object.origin_set(type='ORIGIN_GEOMETRY', center='BOUNDS')

            for level in range(self.lod_levels):
                ratio = self.decimate_ratio ** (level + 1)
                for obj in imported:
                    bpy.ops.object.select_all(action='DESELECT')
                    obj.select_set(True)
                    bpy.context.view_layer.objects.active = obj
                    bpy.ops.object.duplicate()
                    dup = bpy.context.active_object
                    dup.name = f"{name}_LOD{level}"
                    mod = dup.modifiers.new(name="Decimate", type='DECIMATE')
                    mod.ratio = ratio
                    bpy.ops.object.modifier_apply(modifier=mod.name)

                    export_path = os.path.join(EXPORT_DIR, "models", f"{name}_LOD{level}.glb")
                    bpy.ops.export_scene.gltf(filepath=export_path, use_selection=True, export_format='GLB')

                for obj in imported:
                    if obj.modifiers:
                        bpy.ops.object.modifier_apply(modifier=obj.modifiers[0].name)

        self.report({'INFO'}, f"Imported {name} with {self.lod_levels} LODs")
        return {'FINISHED'}

    def _optimize_object(self, obj):
        if obj.data and hasattr(obj.data, 'polygons'):
            tri_count = len(obj.data.polygons)
            if tri_count > 5000:
                mod = obj.modifiers.new(name="AutoDecimate", type='DECIMATE')
                mod.ratio = min(5000.0 / tri_count, 0.8)
                bpy.ops.object.modifier_apply(modifier=mod.name)

        for mat_slot in obj.material_slots:
            if mat_slot.material:
                mat = mat_slot.material
                mat.use_nodes = True
                nodes = mat.node_tree.nodes
                for node in nodes:
                    if node.type == 'BSDF_PRINCIPLED':
                        node.inputs['Roughness'].default_value = min(node.inputs['Roughness'].default_value, 0.9)

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}

class WASTELAND_OT_export_godot_scene(bpy.types.Operator):
    bl_idname = "wasteland.export_godot_scene"
    bl_label = "Export to Godot"
    bl_description = "Export all selected objects as GLTF for Godot with proper naming"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        ensure_export_dir()
        selected = [o for o in bpy.context.selected_objects]
        if not selected:
            self.report({'WARNING'}, "No objects selected")
            return {'CANCELLED'}

        export_count = 0
        for obj in selected:
            bpy.ops.object.select_all(action='DESELECT')
            obj.select_set(True)
            bpy.context.view_layer.objects.active = obj

            name = obj.name.replace(" ", "_").replace(".", "_")
            path = os.path.join(EXPORT_DIR, "models", f"{name}.glb")
            bpy.ops.export_scene.gltf(
                filepath=path, use_selection=True,
                export_format='GLB', export_apply=True,
                export_image_format='JPEG'
            )
            export_count += 1

        self.report({'INFO'}, f"Exported {export_count} objects to {EXPORT_DIR}")
        return {'FINISHED'}

class WASTELAND_PT_panel(bpy.types.Panel):
    bl_label = "Wasteland Pipeline"
    bl_idname = "WASTELAND_PT_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Wasteland"

    def draw(self, context):
        layout = self.layout
        layout.label(text="Asset Generation", icon='MESH_DATA')
        row = layout.row()
        row.operator("wasteland.generate_forest_assets", text="Generate Forest", icon='FOREST')
        row = layout.row()
        row.operator("wasteland.generate_building_kit", text="Building Kit", icon='HOME')

        layout.separator()
        layout.label(text="Import & Optimize", icon='IMPORT')
        row = layout.row()
        row.operator("wasteland.import_and_cleanup", text="Import Model", icon='FILE')

        layout.separator()
        layout.label(text="Export", icon='EXPORT')
        row = layout.row()
        row.operator("wasteland.export_godot_scene", text="Export to Godot", icon='GAME')

        layout.separator()
        box = layout.box()
        box.label(text=f"Export Dir: {EXPORT_DIR}", icon='DISK_DRIVE')

classes = [
    WASTELAND_OT_generate_forest_assets,
    WASTELAND_OT_generate_building_kit,
    WASTELAND_OT_import_and_cleanup,
    WASTELAND_OT_export_godot_scene,
    WASTELAND_PT_panel,
]

def register():
    for cls in classes:
        bpy.utils.register_class(cls)

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)

if __name__ == "__main__":
    register()