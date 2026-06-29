extends Node3D
class_name LSystemTree

var tree_count: int = 150
var spawn_radius: float = 230.0
var spawn_seed: int = 777

func _ready():
	generate_forest()

func generate_forest():
	var rng = RandomNumberGenerator.new()
	rng.seed = spawn_seed

	for i in range(tree_count):
		var angle = rng.randf() * TAU
		var dist = rng.randf_range(20.0, spawn_radius)
		var x = cos(angle) * dist
		var z = sin(angle) * dist

		var tree = _generate_tree(rng)
		tree.position = Vector3(x, 0, z)
		add_child(tree)

func _generate_tree(rng: RandomNumberGenerator) -> Node3D:
	var tree = Node3D.new()
	tree.name = "Tree"

	var tree_mat = StandardMaterial3D.new()
	tree_mat.albedo_color = Color(0.25 + rng.randf() * 0.1, 0.12 + rng.randf() * 0.05, 0.05 + rng.randf() * 0.03)
	tree_mat.roughness = 0.9
	tree_mat.metallic = 0.0

	var leaf_mat = StandardMaterial3D.new()
	leaf_mat.albedo_color = Color(0.15 + rng.randf() * 0.15, 0.25 + rng.randf() * 0.2, 0.05 + rng.randf() * 0.1)
	leaf_mat.roughness = 0.8
	leaf_mat.metallic = 0.0

	var trunk_height = rng.randf_range(3.0, 8.0)
	var trunk_radius = rng.randf_range(0.2, 0.5)

	var trunk = _make_cylinder(Vector3(0, trunk_height / 2, 0), trunk_radius * 0.6, trunk_radius, trunk_height)
	var trunk_node = MeshInstance3D.new()
	trunk_node.mesh = trunk
	trunk_node.material_override = tree_mat
	tree.add_child(trunk_node)

	var branch_count = rng.randi_range(2, 5)
	for b in range(branch_count):
		var branch = _generate_branch(rng, trunk_height, trunk_radius, tree_mat)
		tree.add_child(branch)

	var foliage_count = rng.randi_range(3, 8)
	for f in range(foliage_count):
		var foliage = _generate_foliage(rng, trunk_height, leaf_mat)
		tree.add_child(foliage)

	return tree

func _make_cylinder(pos: Vector3, radius_top: float, radius_bottom: float, height: float) -> CylinderMesh:
	var mesh = CylinderMesh.new()
	mesh.top_radius = radius_top
	mesh.bottom_radius = radius_bottom
	mesh.height = height
	return mesh

func _generate_branch(rng: RandomNumberGenerator, trunk_height: float, _trunk_radius: float, mat: Material) -> Node3D:
	var branch = Node3D.new()
	branch.name = "Branch"

	var start_y = trunk_height * rng.randf_range(0.3, 0.9)
	var length = rng.randf_range(1.5, 4.0)
	var angle = rng.randf_range(20.0, 60.0)
	var direction = rng.randf_range(0.0, TAU)

	var branch_mesh = CylinderMesh.new()
	branch_mesh.top_radius = 0.05
	branch_mesh.bottom_radius = 0.12
	branch_mesh.height = length

	var mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = branch_mesh
	mesh_instance.material_override = mat
	mesh_instance.position = Vector3(0, start_y, 0)
	mesh_instance.rotation_degrees.x = angle
	mesh_instance.rotation_degrees.y = rad_to_deg(direction)

	branch.add_child(mesh_instance)
	return branch

func _generate_foliage(rng: RandomNumberGenerator, trunk_height: float, mat: Material) -> Node3D:
	var foliage = Node3D.new()
	foliage.name = "Foliage"

	var cluster_count = rng.randi_range(1, 3)
	for c in range(cluster_count):
		var sphere = SphereMesh.new()
		sphere.radius = rng.randf_range(0.8, 2.0)
		sphere.height = sphere.radius * 2
		sphere.radial_segments = 6
		sphere.rings = 4

		var mesh_instance = MeshInstance3D.new()
		mesh_instance.mesh = sphere
		mesh_instance.material_override = mat

		var offset_x = rng.randf_range(-1.5, 1.5)
		var offset_z = rng.randf_range(-1.5, 1.5)
		var y = trunk_height + rng.randf_range(-0.5, 2.0)
		mesh_instance.position = Vector3(offset_x, y, offset_z)

		foliage.add_child(mesh_instance)

	return foliage