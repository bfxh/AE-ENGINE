import bpy
import sys
import os
import math

sys.path.insert(0, os.path.dirname(__file__))
from wasteland_generator import generate_terrain, generate_buildings, generate_trees

class Props:
    terrain_size = 500.0
    terrain_resolution = 128
    height_scale = 30.0
    noise_seed = 42
    building_count = 30
    building_radius = 220.0
    tree_count = 100
    tree_radius = 230.0

props = Props()

print("Generating scene...")
generate_terrain(props)
generate_buildings(props)
generate_trees(props)

print("Setting up lighting and camera...")
bpy.context.scene.render.engine = 'BLENDER_EEVEE'
bpy.context.scene.render.resolution_x = 1920
bpy.context.scene.render.resolution_y = 1080

bpy.context.scene.world.use_nodes = True
bg = bpy.context.scene.world.node_tree.nodes['Background']
bg.inputs['Color'].default_value = (0.05, 0.03, 0.02, 1.0)
bg.inputs['Strength'].default_value = 0.1

sun_data = bpy.data.lights.new("Sun_Light", 'SUN')
sun_data.energy = 2.5
sun_data.color = (1.0, 0.85, 0.6)
sun = bpy.data.objects.new("Sun", sun_data)
bpy.context.collection.objects.link(sun)
sun.rotation_euler = (math.radians(45), math.radians(30), 0)

cam_data = bpy.data.cameras.new("Camera")
cam_data.lens = 35
cam = bpy.data.objects.new("Camera_View", cam_data)
bpy.context.collection.objects.link(cam)
cam.location = (80, -180, 100)
cam.rotation_euler = (math.radians(60), math.radians(0), math.radians(25))
bpy.context.scene.camera = cam

output_dir = os.path.dirname(__file__)
output_path = os.path.join(output_dir, "wasteland_render.png")
bpy.context.scene.render.filepath = output_path
bpy.context.scene.render.image_settings.file_format = 'PNG'

print(f"Rendering to {output_path}...")
bpy.ops.render.render(write_still=True)

total_verts = sum(len(o.data.vertices) for o in bpy.data.objects if o.type == 'MESH')
print(f"RENDER COMPLETE: {total_verts:,} vertices -> {output_path}")
bpy.ops.wm.quit_blender()