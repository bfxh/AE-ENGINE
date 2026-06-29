extends Node3D
class_name AdvancedTreeSystem

var tree_count: int = 300
var spawn_radius: float = 230.0
var spawn_seed: int = 42
var terrain: TerrainGenerator = null
var tree_parent: Node3D

enum TreeSpecies { PINE, OAK, BIRCH, WILLOW, DEAD, BURNT }

var species_config = {
	TreeSpecies.PINE: {
		"trunk_height_min": 6.0, "trunk_height_max": 15.0,
		"trunk_radius_min": 0.3, "trunk_radius_max": 0.7,
		"branch_min": 3, "branch_max": 7,
		"foliage_min": 4, "foliage_max": 10,
		"trunk_color": Color(0.35, 0.22, 0.1),
		"leaf_color": Color(0.1, 0.35, 0.1),
		"probability": 0.35
	},
	TreeSpecies.OAK: {
		"trunk_height_min": 5.0, "trunk_height_max": 10.0,
		"trunk_radius_min": 0.4, "trunk_radius_max": 0.9,
		"branch_min": 4, "branch_max": 8,
		"foliage_min": 6, "foliage_max": 14,
		"trunk_color": Color(0.3, 0.18, 0.08),
		"leaf_color": Color(0.15, 0.3, 0.1),
		"probability": 0.25
	},
	TreeSpecies.BIRCH: {
		"trunk_height_min": 8.0, "trunk_height_max": 14.0,
		"trunk_radius_min": 0.2, "trunk_radius_max": 0.4,
		"branch_min": 2, "branch_max": 5,
		"foliage_min": 3, "foliage_max": 7,
		"trunk_color": Color(0.75, 0.7, 0.6),
		"leaf_color": Color(0.2, 0.4, 0.1),
		"probability": 0.2
	},
	TreeSpecies.WILLOW: {
		"trunk_height_min": 4.0, "trunk_height_max": 8.0,
		"trunk_radius_min": 0.3, "trunk_radius_max": 0.6,
		"branch_min": 5, "branch_max": 10,
		"foliage_min": 8, "foliage_max": 16,
		"trunk_color": Color(0.3, 0.2, 0.1),
		"leaf_color": Color(0.25, 0.45, 0.15),
		"probability": 0.1
	},
	TreeSpecies.DEAD: {
		"trunk_height_min": 3.0, "trunk_height_max": 10.0,
		"trunk_radius_min": 0.2, "trunk_radius_max": 0.6,
		"branch_min": 1, "branch_max": 4,
		"foliage_min": 0, "foliage_max": 2,
		"trunk_color": Color(0.35, 0.3, 0.25),
		"leaf_color": Color(0.3, 0.25, 0.15),
		"probability": 0.07
	},
	TreeSpecies.BURNT: {
		"trunk_height_min": 2.0, "trunk_height_max": 8.0,
		"trunk_radius_min": 0.2, "trunk_radius_max": 0.5,
		"branch_min": 0, "branch_max": 2,
		"foliage_min": 0, "foliage_max": 1,
		"trunk_color": Color(0.1, 0.08, 0.06),
		"leaf_color": Color(0.15, 0.1, 0.05),
		"probability": 0.03
	}
}

func _ready():
	tree_parent = Node3D.new()
	tree_parent.name = "Forest"
	add_child(tree_parent)
	generate_forest()

func set_terrain(p_terrain: TerrainGenerator):
	terrain = p_terrain

func generate_forest():
	var rng = RandomNumberGenerator.new()
	rng.seed = spawn_seed

	for i in range(tree_count):
		var species = _pick_species(rng)
		var pos = _find_valid_position(rng)
		if pos != null:
			var tree = _generate_tree(species, rng)
			tree.position = pos
			tree_parent.add_child(tree)

	print("[Forest] Generated %d trees (%d species)" % [tree_count, TreeSpecies.size()])

func _pick_species(rng: RandomNumberGenerator) -> TreeSpecies:
	var roll = rng.randf()
	var cumulative = 0.0
	for species in TreeSpecies.values():
		var config = species_config[species]
		cumulative += config["probability"]
		if roll <= cumulative:
			return species
	return TreeSpecies.PINE

func _find_valid_position(rng: RandomNumberGenerator):
	var max_attempts = 10
	for attempt in range(max_attempts):
		var angle = rng.randf() * TAU
		var dist = rng.randf_range(15.0, spawn_radius)
		var x = cos(angle) * dist
		var z = sin(angle) * dist
		var y = 0.0

		if terrain:
			var ray_query = PhysicsRayQueryParameters3D.create(
				Vector3(x, 500, z), Vector3(x, -50, z)
			)
			var space_state = get_world_3d().direct_space_state
			if space_state:
				var result = space_state.intersect_ray(ray_query)
				if result:
					y = result.position.y
			y += 0.5
		else:
			y = 0.0

		if y < -20 or y > 80:
			continue

		var slope_ok = true
		if terrain and attempt < 5:
			for dx in [-2, 0, 2]:
				for dz in [-2, 0, 2]:
					var px = x + dx
					var pz = z + dz
					var h = 0.0
					var rq2 = PhysicsRayQueryParameters3D.create(Vector3(px, 500, pz), Vector3(px, -50, pz))
					var ss = get_world_3d().direct_space_state
					if ss:
						var rr = ss.intersect_ray(rq2)
						if rr:
							h = rr.position.y
					if abs(h - y) > 5:
						slope_ok = false

		if slope_ok:
			return Vector3(x, y, z)
	return null

func _generate_tree(species: TreeSpecies, rng: RandomNumberGenerator) -> Node3D:
	var tree = Node3D.new()
	tree.name = TreeSpecies.keys()[species]

	var config = species_config[species]
	var trunk_h = rng.randf_range(config["trunk_height_min"], config["trunk_height_max"])
	var trunk_r = rng.randf_range(config["trunk_radius_min"], config["trunk_radius_max"])

	var trunk_mat = StandardMaterial3D.new()
	trunk_mat.albedo_color = config["trunk_color"].lightened(rng.randf_range(-0.05, 0.05))
	trunk_mat.roughness = 0.85
	trunk_mat.metallic = 0.0

	var leaf_mat = StandardMaterial3D.new()
	leaf_mat.albedo_color = config["leaf_color"].lightened(rng.randf_range(-0.08, 0.08))
	leaf_mat.roughness = 0.75
	leaf_mat.metallic = 0.0

	var trunk_mesh = CylinderMesh.new()
	trunk_mesh.top_radius = trunk_r * 0.5
	trunk_mesh.bottom_radius = trunk_r
	trunk_mesh.height = trunk_h

	var trunk_node = MeshInstance3D.new()
	trunk_node.mesh = trunk_mesh
	trunk_node.material_override = trunk_mat
	trunk_node.position = Vector3(0, trunk_h / 2, 0)
	tree.add_child(trunk_node)

	var branch_count = rng.randi_range(config["branch_min"], config["branch_max"])
	for b in range(branch_count):
		var branch = _generate_branch(species, rng, trunk_h, trunk_r, trunk_mat)
		tree.add_child(branch)

	var foliage_count = rng.randi_range(config["foliage_min"], config["foliage_max"])
	for f in range(foliage_count):
		var foliage = _generate_foliage(species, rng, trunk_h, leaf_mat)
		tree.add_child(foliage)

	if species == TreeSpecies.PINE:
		var pine_top = _generate_pine_top(rng, trunk_h, trunk_r, leaf_mat)
		tree.add_child(pine_top)

	return tree

func _generate_branch(_species: TreeSpecies, rng: RandomNumberGenerator, trunk_h: float, _trunk_r: float, mat: Material) -> Node3D:
	var branch = Node3D.new()
	branch.name = "Branch"

	var start_y = trunk_h * rng.randf_range(0.2, 0.85)
	var length = rng.randf_range(1.5, 5.0)
	var angle_x = rng.randf_range(15.0, 55.0)
	var angle_y = rng.randf_range(0.0, 360.0)

	var bmesh = CylinderMesh.new()
	bmesh.top_radius = 0.04
	bmesh.bottom_radius = 0.1
	bmesh.height = length

	var mi = MeshInstance3D.new()
	mi.mesh = bmesh
	mi.material_override = mat
	mi.position = Vector3(0, start_y, 0)
	mi.rotation_degrees = Vector3(angle_x, 0, 0)
	mi.rotate(Vector3.UP, deg_to_rad(angle_y))

	branch.add_child(mi)

	if rng.randf() > 0.6:
		var sub_branch = Node3D.new()
		sub_branch.name = "SubBranch"
		var sl = rng.randf_range(1.0, 2.5)
		var sbm = CylinderMesh.new()
		sbm.top_radius = 0.02
		sbm.bottom_radius = 0.06
		sbm.height = sl
		var smi = MeshInstance3D.new()
		smi.mesh = sbm
		smi.material_override = mat
		smi.position = Vector3(0, length * 0.8, 0)
		smi.rotation_degrees.x = rng.randf_range(20, 50)
		smi.rotation_degrees.y = rng.randf_range(0, 360)
		sub_branch.add_child(smi)
		branch.add_child(sub_branch)

	return branch

func _generate_foliage(_species: TreeSpecies, rng: RandomNumberGenerator, trunk_h: float, mat: Material) -> Node3D:
	var foliage = Node3D.new()
	foliage.name = "Foliage"

	var cluster_count = rng.randi_range(1, 4)
	for c in range(cluster_count):
		var sphere = SphereMesh.new()
		sphere.radius = rng.randf_range(0.6, 2.2)
		sphere.height = sphere.radius * 2
		sphere.radial_segments = 7
		sphere.rings = 5

		var mi = MeshInstance3D.new()
		mi.mesh = sphere
		mi.material_override = mat

		var ox = rng.randf_range(-2.0, 2.0)
		var oz = rng.randf_range(-2.0, 2.0)
		var oy = trunk_h + rng.randf_range(-1.0, 3.0)
		mi.position = Vector3(ox, oy, oz)

		var scale_var = rng.randf_range(0.7, 1.3)
		mi.scale = Vector3(scale_var, scale_var * 0.8, scale_var)

		foliage.add_child(mi)

	return foliage

func _generate_pine_top(rng: RandomNumberGenerator, trunk_h: float, _trunk_r: float, mat: Material) -> Node3D:
	var top = Node3D.new()
	top.name = "PineTop"

	var cone_count = rng.randi_range(2, 4)
	for i in range(cone_count):
		var cone_mesh = CylinderMesh.new()
		var cr = 1.0 - float(i) * 0.25
		cone_mesh.top_radius = 0.0
		cone_mesh.bottom_radius = cr * 0.8
		cone_mesh.height = 2.5

		var mi = MeshInstance3D.new()
		mi.mesh = cone_mesh
		mi.material_override = mat
		mi.position = Vector3(0, trunk_h + float(i) * 1.5, 0)
		top.add_child(mi)

	return top