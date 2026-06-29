bl_info = {
    "name": "Wasteland Generator",
    "author": "Wasteland Team",
    "version": (0, 2, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > Wasteland",
    "description": "Procedural wasteland environment generator - terrain, buildings, trees",
    "category": "3D View",
}

import bpy
import bmesh
import math
import random
from mathutils import Vector, Matrix, noise

class WastelandProperties(bpy.types.PropertyGroup):
    terrain_size: bpy.props.FloatProperty(name="Terrain Size", default=500.0, min=50.0, max=2000.0)
    terrain_resolution: bpy.props.IntProperty(name="Resolution", default=128, min=32, max=512)
    height_scale: bpy.props.FloatProperty(name="Height Scale", default=30.0, min=1.0, max=100.0)
    noise_seed: bpy.props.IntProperty(name="Seed", default=42, min=0, max=10000)

    building_count: bpy.props.IntProperty(name="Building Count", default=30, min=1, max=200)
    building_radius: bpy.props.FloatProperty(name="Spawn Radius", default=220.0, min=10.0, max=500.0)

    tree_count: bpy.props.IntProperty(name="Tree Count", default=100, min=1, max=500)
    tree_radius: bpy.props.FloatProperty(name="Spawn Radius", default=230.0, min=10.0, max=500.0)

    export_path: bpy.props.StringProperty(name="Export Path", default="//wasteland_assets.glb", subtype='FILE_PATH')

class WASTELAND_OT_generate_terrain(bpy.types.Operator):
    bl_idname = "wasteland.generate_terrain"
    bl_label = "Generate Terrain"
    bl_description = "Generate procedural wasteland terrain"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        props = context.scene.wasteland_props
        generate_terrain(props)
        return {'FINISHED'}

class WASTELAND_OT_generate_buildings(bpy.types.Operator):
    bl_idname = "wasteland.generate_buildings"
    bl_label = "Generate Buildings"
    bl_description = "Generate procedural wasteland buildings"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        props = context.scene.wasteland_props
        generate_buildings(props)
        return {'FINISHED'}

class WASTELAND_OT_generate_trees(bpy.types.Operator):
    bl_idname = "wasteland.generate_trees"
    bl_label = "Generate Trees"
    bl_description = "Generate procedural wasteland trees"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        props = context.scene.wasteland_props
        generate_trees(props)
        return {'FINISHED'}

class WASTELAND_OT_generate_all(bpy.types.Operator):
    bl_idname = "wasteland.generate_all"
    bl_label = "Generate All"
    bl_description = "Generate complete wasteland scene"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        props = context.scene.wasteland_props
        generate_terrain(props)
        generate_buildings(props)
        generate_trees(props)
        return {'FINISHED'}

class WASTELAND_OT_export_glb(bpy.types.Operator):
    bl_idname = "wasteland.export_glb"
    bl_label = "Export to glTF"
    bl_description = "Export scene to glTF for Godot"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        props = context.scene.wasteland_props
        path = bpy.path.abspath(props.export_path)
        bpy.ops.export_scene.gltf(
            filepath=path,
            export_format='GLB',
            export_apply=True,
            export_image_format='NONE',
            export_texcoords=True,
            export_normals=True,
        )
        self.report({'INFO'}, f"Exported to {path}")
        return {'FINISHED'}

class WASTELAND_OT_clear_scene(bpy.types.Operator):
    bl_idname = "wasteland.clear_scene"
    bl_label = "Clear Wasteland"
    bl_description = "Remove all wasteland objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        for obj in list(bpy.data.objects):
            if obj.name.startswith("Wasteland_"):
                bpy.data.objects.remove(obj, do_unlink=True)
        self.report({'INFO'}, "Cleared all wasteland objects")
        return {'FINISHED'}

class WASTELAND_PT_panel(bpy.types.Panel):
    bl_label = "Wasteland Generator"
    bl_idname = "WASTELAND_PT_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Wasteland"

    def draw(self, context):
        layout = self.layout
        props = context.scene.wasteland_props

        box = layout.box()
        box.label(text="Terrain", icon='MESH_GRID')
        box.prop(props, "terrain_size")
        box.prop(props, "terrain_resolution")
        box.prop(props, "height_scale")
        box.prop(props, "noise_seed")
        box.operator("wasteland.generate_terrain", icon='MESH_GRID')

        box = layout.box()
        box.label(text="Buildings", icon='MESH_CUBE')
        box.prop(props, "building_count")
        box.prop(props, "building_radius")
        box.operator("wasteland.generate_buildings", icon='MESH_CUBE')

        box = layout.box()
        box.label(text="Trees", icon='OUTLINER_OB_HAIR')
        box.prop(props, "tree_count")
        box.prop(props, "tree_radius")
        box.operator("wasteland.generate_trees", icon='OUTLINER_OB_HAIR')

        layout.separator()
        layout.operator("wasteland.generate_all", icon='EVENT_PLUS')
        layout.operator("wasteland.clear_scene", icon='TRASH')

        box = layout.box()
        box.label(text="Export", icon='EXPORT')
        box.prop(props, "export_path")
        box.operator("wasteland.export_glb", icon='EXPORT')


def create_material(name, color, roughness=0.85, metallic=0.0):
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = (*color, 1.0)
        bsdf.inputs["Roughness"].default_value = roughness
        bsdf.inputs["Metallic"].default_value = metallic
    return mat

def wasteland_noise(x, y, seed=0, octaves=4):
    value = 0.0
    amp = 1.0
    freq = 0.005
    for i in range(octaves):
        nx = x * freq + seed * 123
        nz = y * freq + seed * 456
        value += noise.noise(Vector((nx, nz, 0.0))) * amp
        amp *= 0.5
        freq *= 2.0
    return value

def generate_terrain(props):
    size = props.terrain_size
    res = props.terrain_resolution
    height = props.height_scale
    seed = props.noise_seed

    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=res, y_segments=res, size=size)

    for v in bm.verts:
        x, y = v.co.x, v.co.y
        h1 = wasteland_noise(x, y, seed, 4) * height
        h2 = wasteland_noise(x, y, seed + 100, 3) * height * 0.3
        h3 = wasteland_noise(x, y, seed + 200, 2) * height * 0.5
        h = h1 + h2 + h3
        h += abs(wasteland_noise(x * 0.3, y * 0.3, seed + 50, 2)) * height * 0.2
        v.co.z = h

    mesh = bpy.data.meshes.new("Wasteland_Terrain_Mesh")
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new("Wasteland_Terrain", mesh)
    bpy.context.collection.objects.link(obj)

    mat = create_material("Wasteland_Ground", (0.35, 0.28, 0.18), roughness=0.9)
    obj.data.materials.append(mat)

    bpy.ops.object.select_all(action='DESELECT')
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj

    bpy.ops.object.shade_smooth()

    return obj

def make_building_box(props, w, d, h):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)

    for v in bm.verts:
        v.co.x *= w / 2
        v.co.y *= d / 2
        v.co.z *= 1.0
        if v.co.z < 0:
            v.co.z = 0
        else:
            v.co.z = h

    return bm

def make_building_house(props, w, d, h):
    bm = bmesh.new()

    hw = w / 2
    hd = d / 2
    roof_h = h + random.uniform(2.0, 4.0)

    verts = [
        bm.verts.new((-hw, -hd, 0)), bm.verts.new((hw, -hd, 0)),
        bm.verts.new((hw, hd, 0)), bm.verts.new((-hw, hd, 0)),
        bm.verts.new((-hw, -hd, h)), bm.verts.new((hw, -hd, h)),
        bm.verts.new((hw, hd, h)), bm.verts.new((-hw, hd, h)),
    ]
    bm.verts.ensure_lookup_table()

    faces = [
        (0, 1, 2, 3), (0, 1, 5, 4), (1, 2, 6, 5),
        (2, 3, 7, 6), (3, 0, 4, 7),
    ]
    for f in faces:
        bm.faces.new([verts[i] for i in f])

    ridge_verts = [
        bm.verts.new((-hw, -hd/2, roof_h)),
        bm.verts.new((hw, -hd/2, roof_h)),
        bm.verts.new((0, -hd, roof_h + 2)),
        bm.verts.new((0, hd, roof_h + 2)),
    ]
    bm.verts.ensure_lookup_table()

    roof_faces = [
        (verts[4], verts[5], ridge_verts[0]),
        (verts[5], verts[6], ridge_verts[1]),
        (verts[6], verts[7], ridge_verts[1]),
        (verts[7], verts[4], ridge_verts[0]),
        (verts[4], ridge_verts[0], ridge_verts[2]),
        (verts[5], ridge_verts[0], ridge_verts[2]),
        (verts[6], ridge_verts[1], ridge_verts[3]),
        (verts[7], ridge_verts[1], ridge_verts[3]),
    ]
    for f in roof_faces:
        bm.faces.new([v for v in f])

    return bm

def make_building_ruin(props, w, d, h):
    bm = bmesh.new()
    hw = w / 2
    hd = d / 2

    verts = [
        bm.verts.new((-hw, -hd, 0)), bm.verts.new((hw, -hd, 0)),
        bm.verts.new((hw, hd, 0)), bm.verts.new((-hw, hd, 0)),
        bm.verts.new((-hw, -hd, h)), bm.verts.new((hw, -hd, h)),
        bm.verts.new((hw, hd, h)), bm.verts.new((-hw, hd, h)),
    ]
    bm.verts.ensure_lookup_table()

    faces = [
        (0, 1, 2, 3), (0, 1, 5, 4), (1, 2, 6, 5),
    ]
    for f in faces:
        bm.faces.new([verts[i] for i in f])

    return bm

def generate_buildings(props):
    count = props.building_count
    radius = props.building_radius

    mats = [
        create_material("Wasteland_Concrete", (0.5, 0.48, 0.45), roughness=0.85, metallic=0.05),
        create_material("Wasteland_RustedMetal", (0.45, 0.25, 0.1), roughness=0.7, metallic=0.3),
        create_material("Wasteland_Brick", (0.4, 0.2, 0.12), roughness=0.9, metallic=0.0),
    ]

    for i in range(count):
        angle = random.uniform(0, math.pi * 2)
        dist = random.uniform(30.0, radius)
        x = math.cos(angle) * dist
        y = math.sin(angle) * dist

        building_type = random.randint(0, 5)
        bm = None

        if building_type == 0:
            bm = make_building_box(props, random.uniform(3, 6), random.uniform(3, 6), random.uniform(15, 40))
        elif building_type == 1:
            bm = make_building_house(props, random.uniform(6, 12), random.uniform(5, 10), random.uniform(4, 8))
        elif building_type == 2:
            bm = make_building_box(props, random.uniform(10, 20), random.uniform(8, 15), random.uniform(5, 10))
        else:
            bm = make_building_ruin(props, random.uniform(4, 10), random.uniform(4, 8), random.uniform(2, 6))

        if bm is None:
            continue

        mesh = bpy.data.meshes.new(f"Wasteland_Building_{i}_Mesh")
        bm.to_mesh(mesh)
        bm.free()

        obj = bpy.data.objects.new(f"Wasteland_Building_{i}", mesh)
        obj.location = (x, y, 0)
        obj.rotation_euler.z = random.uniform(0, math.pi * 2)
        obj.data.materials.append(random.choice(mats))

        bpy.context.collection.objects.link(obj)

def generate_tree(props):
    trunk_h = random.uniform(3.0, 8.0)
    trunk_r = random.uniform(0.2, 0.5)

    bm = bmesh.new()

    bmesh.ops.create_cone(
        bm,
        cap_ends=True,
        segments=6,
        radius1=trunk_r,
        radius2=trunk_r * 0.4,
        depth=trunk_h
    )

    tree_mesh = bpy.data.meshes.new(f"Wasteland_Tree_Trunk_Mesh")
    bm.to_mesh(tree_mesh)

    trunk_mat = create_material("Wasteland_Bark",
        (0.25 + random.uniform(0, 0.1), 0.12 + random.uniform(0, 0.05), 0.05 + random.uniform(0, 0.03)),
        roughness=0.9)

    obj = bpy.data.objects.new(f"Wasteland_Tree", tree_mesh)
    obj.data.materials.append(trunk_mat)

    branch_count = random.randint(2, 5)
    for b in range(branch_count):
        start_y = trunk_h * random.uniform(0.3, 0.9)
        length = random.uniform(1.5, 4.0)
        angle = random.uniform(20, 60)
        direction = random.uniform(0, math.pi * 2)

        branch_bm = bmesh.new()
        bmesh.ops.create_cone(branch_bm, cap_ends=True, segments=4,
            radius1=0.12, radius2=0.05, depth=length)

        branch_mesh = bpy.data.meshes.new(f"Wasteland_Branch_Mesh")
        branch_bm.to_mesh(branch_mesh)
        branch_bm.free()

        branch_obj = bpy.data.objects.new("Branch", branch_mesh)
        branch_obj.data.materials.append(trunk_mat)
        branch_obj.parent = obj
        branch_obj.location = (0, 0, start_y)
        branch_obj.rotation_euler.x = math.radians(90 - angle)
        branch_obj.rotation_euler.z = direction

        bpy.context.collection.objects.link(branch_obj)

    foliage_count = random.randint(3, 8)
    leaf_mat = create_material("Wasteland_Leaves",
        (0.15 + random.uniform(0, 0.15), 0.25 + random.uniform(0, 0.2), 0.05 + random.uniform(0, 0.1)),
        roughness=0.8)

    for f in range(foliage_count):
        cluster_count = random.randint(1, 3)
        for c in range(cluster_count):
            leaf_bm = bmesh.new()
            radius = random.uniform(0.8, 2.0)
            bmesh.ops.create_uvsphere(leaf_bm, u_segments=6, v_segments=4, radius=radius)

            leaf_mesh = bpy.data.meshes.new(f"Wasteland_Foliage_Mesh")
            leaf_bm.to_mesh(leaf_mesh)
            leaf_bm.free()

            leaf_obj = bpy.data.objects.new("Foliage", leaf_mesh)
            leaf_obj.data.materials.append(leaf_mat)
            leaf_obj.parent = obj
            leaf_obj.location = (
                random.uniform(-1.5, 1.5),
                random.uniform(-1.5, 1.5),
                trunk_h + random.uniform(-0.5, 2.0)
            )

            bpy.context.collection.objects.link(leaf_obj)

    return obj

def generate_trees(props):
    count = props.tree_count
    radius = props.tree_radius

    for i in range(count):
        angle = random.uniform(0, math.pi * 2)
        dist = random.uniform(20.0, radius)
        x = math.cos(angle) * dist
        y = math.sin(angle) * dist

        obj = generate_tree(props)
        obj.location = (x, y, 0)

classes = [
    WastelandProperties,
    WASTELAND_OT_generate_terrain,
    WASTELAND_OT_generate_buildings,
    WASTELAND_OT_generate_trees,
    WASTELAND_OT_generate_all,
    WASTELAND_OT_export_glb,
    WASTELAND_OT_clear_scene,
    WASTELAND_PT_panel,
]

def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Scene.wasteland_props = bpy.props.PointerProperty(type=WastelandProperties)

def unregister():
    for cls in classes:
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.wasteland_props

if __name__ == "__main__":
    register()