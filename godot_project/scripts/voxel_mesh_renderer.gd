extends Node3D
class_name VoxelMeshRenderer

var wasteland_world
var multimesh: MultiMeshInstance3D
var voxel_size: float = 1.0
var update_interval: int = 5
var frame_counter: int = 0

func _init(p_world, p_voxel_size: float = 1.0):
	wasteland_world = p_world
	voxel_size = p_voxel_size

func _ready():
	multimesh = MultiMeshInstance3D.new()
	multimesh.name = "VoxelMultiMesh"
	add_child(multimesh)

	var mesh = BoxMesh.new()
	mesh.size = Vector3(voxel_size * 0.95, voxel_size * 0.95, voxel_size * 0.95)

	var mm = MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = mesh
	multimesh.multimesh = mm

	var mat = StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.8
	multimesh.material_override = mat

	rebuild_all_grids()

func _process(_delta):
	frame_counter += 1
	if frame_counter % update_interval == 0:
		rebuild_all_grids()

func rebuild_all_grids():
	var grid_count = wasteland_world.voxel_grid_count()
	var all_positions = PackedVector3Array()
	var all_colors = PackedColorArray()

	for i in range(grid_count):
		var positions = wasteland_world.get_voxel_positions(i)
		var colors = wasteland_world.get_voxel_colors(i)
		all_positions.append_array(positions)
		all_colors.append_array(colors)

	var mm = multimesh.multimesh
	mm.instance_count = all_positions.size()

	for j in range(all_positions.size()):
		var t = Transform3D.IDENTITY
		t.origin = all_positions[j]
		mm.set_instance_transform(j, t)
		mm.set_instance_color(j, all_colors[j])