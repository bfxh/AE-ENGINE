extends Node3D
class_name WastelandForestGenerator

const WORLD_SIZE: float = 200.0
const GRID_SIZE: float = 10.0
const MAX_TREES: int = 200
const MAX_ROCKS: int = 100
const MAX_VEGETATION: int = 500
const WATER_BODIES: int = 3

var rng := RandomNumberGenerator.new()
var world_seed: int = 42

var terrain: Node3D
var trees: Node3D
var rocks: Node3D
var vegetation: Node3D
var water: Node3D
var animals: Node3D

func _ready():
	set_process(false)
	set_physics_process(false)

func generate(seed: int):
	world_seed = seed
	rng.seed = seed

	_clear_all()
	_create_containers()
	_generate_terrain()
	_generate_trees()
	_generate_rocks()
	_generate_vegetation()
	_generate_water()
	_generate_animals()

	print("[WastelandForest] Generated: Trees=%d, Rocks=%d, Veg=%d" % [trees.get_child_count(), rocks.get_child_count(), vegetation.get_child_count()])

func _create_containers():
	terrain = Node3D.new()
	terrain.name = "Terrain"
	add_child(terrain)

	trees = Node3D.new()
	trees.name = "Trees"
	add_child(trees)

	rocks = Node3D.new()
	rocks.name = "Rocks"
	add_child(rocks)

	vegetation = Node3D.new()
	vegetation.name = "Vegetation"
	add_child(vegetation)

	water = Node3D.new()
	water.name = "Water"
	add_child(water)

	animals = Node3D.new()
	animals.name = "Animals"
	add_child(animals)

func _clear_all():
	for child in get_children():
		child.queue_free()

func _generate_terrain():
	var ground_size = Vector3(WORLD_SIZE * 2.0, 0.2, WORLD_SIZE * 2.0)
	var ground_mesh = BoxMesh.new()
	ground_mesh.size = ground_size

	var ground_mat = StandardMaterial3D.new()
	ground_mat.albedo_color = Color(0.2, 0.15, 0.08)
	ground_mat.roughness = 0.9

	var ground = MeshInstance3D.new()
	ground.mesh = ground_mesh
	ground.material_override = ground_mat
	ground.name = "Ground"
	terrain.add_child(ground)

	var num_hills = rng.randi_range(3, 8)
	for i in range(num_hills):
		var hill = _create_hill()
		hill.position = Vector3(rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2), 0, rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2))
		terrain.add_child(hill)

func _create_hill() -> MeshInstance3D:
	var size = Vector3(rng.randf_range(10.0, 30.0), rng.randf_range(2.0, 8.0), rng.randf_range(10.0, 30.0))
	var mesh = BoxMesh.new()
	mesh.size = size

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.25, 0.2, 0.1)
	mat.roughness = 0.95

	var hill = MeshInstance3D.new()
	hill.mesh = mesh
	hill.material_override = mat
	hill.position = Vector3(0, size.y/2.0, 0)
	return hill

func _generate_trees():
	for i in range(MAX_TREES):
		var x = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var z = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var y = 0

		var height = rng.randf_range(6.0, 20.0)
		var radius = rng.randf_range(0.3, 0.8)
		var species = rng.randi() % 6

		var tree = _create_tree(species, height, radius)
		tree.position = Vector3(x, y, z)
		tree.rotation = Vector3(0, rng.randf_range(0, TAU), 0)
		tree.scale = Vector3(1, 1, 1) * rng.randf_range(0.7, 1.5)
		trees.add_child(tree)

func _create_tree(species: int, height: float, radius: float) -> Node3D:
	var tree = Node3D.new()
	tree.name = "Tree_%d" % species

	var trunk_height = height * 0.5
	var trunk_radius = radius
	var trunk_mesh = CylinderMesh.new()
	trunk_mesh.top_radius = trunk_radius * 0.8
	trunk_mesh.bottom_radius = trunk_radius * 1.1
	trunk_mesh.height = trunk_height
	trunk_mesh.radial_segments = 8

	var trunk_mat = StandardMaterial3D.new()
	trunk_mat.albedo_color = Color(0.35, 0.22, 0.1)
	trunk_mat.roughness = 0.9

	var trunk = MeshInstance3D.new()
	trunk.mesh = trunk_mesh
	trunk.material_override = trunk_mat
	trunk.position = Vector3(0, trunk_height/2.0, 0)
	tree.add_child(trunk)

	var leaf_height = height * 0.6
	var leaf_radius = radius * 5.0

	match species:
		0: _add_pine_leaves(tree, leaf_height, trunk_height, leaf_radius)
		1: _add_oak_leaves(tree, leaf_height, trunk_height, leaf_radius)
		2: _add_birch_leaves(tree, leaf_height, trunk_height, leaf_radius)
		3: _add_willow_leaves(tree, leaf_height, trunk_height, leaf_radius)
		4: _add_dead_leaves(tree, leaf_height, trunk_height, leaf_radius)
		5: _add_dead_leaves(tree, leaf_height, trunk_height, leaf_radius * 0.6)

	return tree

func _add_pine_leaves(parent: Node3D, leaf_h: float, trunk_h: float, radius: float):
	var layers = rng.randi_range(3, 6)
	var step = leaf_h / layers

	for i in range(layers):
		var y = trunk_h + step * i
		var r = radius * (1.0 - float(i) / layers * 0.4)

		var leaf = MeshInstance3D.new()
		var mesh = CylinderMesh.new()
		mesh.top_radius = r * 0.1
		mesh.bottom_radius = r
		mesh.height = step * 0.8
		mesh.radial_segments = 8
		leaf.mesh = mesh

		var mat = StandardMaterial3D.new()
		mat.albedo_color = Color(0.05, 0.25, 0.05)
		mat.roughness = 0.85
		leaf.material_override = mat
		leaf.position = Vector3(0, y, 0)
		parent.add_child(leaf)

func _add_oak_leaves(parent: Node3D, trunk_h: float, leaf_h: float, radius: float):
	var num_spheres = rng.randi_range(5, 9)
	for i in range(num_spheres):
		var leaf = MeshInstance3D.new()
		var mesh = SphereMesh.new()
		mesh.radial_segments = 8
		mesh.latitudinal_segments = 8
		mesh.radius = radius * (0.6 + rng.randf() * 0.4)
		mesh.height = mesh.radius * 2.0
		leaf.mesh = mesh

		var mat = StandardMaterial3D.new()
		mat.albedo_color = Color(0.1, 0.35, 0.1)
		mat.roughness = 0.8
		leaf.material_override = mat

		var angle = float(i) / num_spheres * TAU
		var dist = radius * (0.2 + rng.randf() * 0.4)
		leaf.position = Vector3(
			cos(angle) * dist,
			trunk_h + leaf_h * 0.6 + rng.randf_range(-0.5, 0.5),
			sin(angle) * dist
		)
		parent.add_child(leaf)

func _add_birch_leaves(parent: Node3D, trunk_h: float, leaf_h: float, radius: float):
	var num_clusters = rng.randi_range(4, 7)
	for i in range(num_clusters):
		var leaf = MeshInstance3D.new()
		var mesh = SphereMesh.new()
		mesh.radial_segments = 6
		mesh.latitudinal_segments = 6
		mesh.radius = radius * (0.4 + rng.randf() * 0.5)
		mesh.height = mesh.radius * 2.0
		leaf.mesh = mesh

		var mat = StandardMaterial3D.new()
		mat.albedo_color = Color(0.4, 0.55, 0.2)
		mat.roughness = 0.85
		leaf.material_override = mat

		var angle = float(i) / num_clusters * TAU
		var dist = radius * 0.4
		leaf.position = Vector3(
			cos(angle) * dist,
			trunk_h + leaf_h * 0.7,
			sin(angle) * dist
		)
		parent.add_child(leaf)

func _add_willow_leaves(parent: Node3D, trunk_h: float, leaf_h: float, radius: float):
	var num_strands = rng.randi_range(8, 15)
	for i in range(num_strands):
		var leaf = MeshInstance3D.new()
		var mesh = CylinderMesh.new()
		mesh.top_radius = radius * 0.05
		mesh.bottom_radius = radius * 0.3
		mesh.height = leaf_h * (0.4 + rng.randf() * 0.4)
		mesh.radial_segments = 4
		leaf.mesh = mesh

		var mat = StandardMaterial3D.new()
		mat.albedo_color = Color(0.15, 0.35, 0.15)
		mat.roughness = 0.9
		leaf.material_override = mat

		var angle = float(i) / num_strands * TAU
		leaf.position = Vector3(
			cos(angle) * radius * 0.6,
			trunk_h + leaf_h,
			sin(angle) * radius * 0.6
		)
		leaf.rotation.x = rng.randf_range(0.8, 1.2)
		leaf.rotation.y = angle
		parent.add_child(leaf)

func _add_dead_leaves(parent: Node3D, trunk_h: float, leaf_h: float, radius: float):
	var num_spikes = rng.randi_range(3, 6)
	for i in range(num_spikes):
		var dead = MeshInstance3D.new()
		var mesh = CylinderMesh.new()
		mesh.top_radius = radius * 0.05
		mesh.bottom_radius = radius * 0.3
		mesh.height = leaf_h * 0.5
		mesh.radial_segments = 4
		dead.mesh = mesh

		var mat = StandardMaterial3D.new()
		mat.albedo_color = Color(0.2, 0.15, 0.08)
		mat.roughness = 0.95
		dead.material_override = mat

		var angle = float(i) / num_spikes * TAU
		dead.position = Vector3(
			cos(angle) * radius * 0.5,
			trunk_h + leaf_h * 0.8,
			sin(angle) * radius * 0.5
		)
		dead.rotation = Vector3(rng.randf_range(-0.5, 0.5), angle, rng.randf_range(-0.5, 0.5))
		parent.add_child(dead)

func _generate_rocks():
	for i in range(MAX_ROCKS):
		var x = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var z = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var size = rng.randf_range(0.3, 2.5)

		var rock = _create_rock(size)
		rock.position = Vector3(x, size/2.0, z)
		rock.rotation = Vector3(rng.randf_range(0, TAU/6), rng.randf_range(0, TAU), rng.randf_range(0, TAU/6))
		rocks.add_child(rock)

func _create_rock(size: float) -> MeshInstance3D:
	var mesh = SphereMesh.new()
	mesh.radial_segments = 8
	mesh.latitudinal_segments = 6
	mesh.radius = size
	mesh.height = size * 1.5

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.4, 0.38, 0.35)
	mat.roughness = 0.95

	var rock = MeshInstance3D.new()
	rock.mesh = mesh
	rock.material_override = mat
	return rock

func _generate_vegetation():
	for i in range(MAX_VEGETATION):
		var x = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var z = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var y = 0

		var veg_type = rng.randi() % 7
		var veg = _create_vegetation(veg_type)
		veg.position = Vector3(x, y, z)
		veg.scale = Vector3(1, 1, 1) * (0.5 + rng.randf() * 0.8)
		vegetation.add_child(veg)

func _create_vegetation(veg_type: int) -> Node3D:
	var veg = Node3D.new()

	match veg_type:
		0: veg = _create_grass()
		1: veg = _create_bush()
		2: veg = _create_mushroom()
		3: veg = _create_fern()
		4: veg = _create_grass()
		5: veg = _create_bush()
		6: veg = _create_flower()

	return veg

func _create_grass() -> Node3D:
	var grass = Node3D.new()
	grass.name = "Grass"

	var height = rng.randf_range(0.3, 0.8)
	var mesh = CylinderMesh.new()
	mesh.top_radius = 0.01
	mesh.bottom_radius = 0.02
	mesh.height = height
	mesh.radial_segments = 3

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.2, 0.45, 0.1)
	mat.roughness = 0.95

	var g_mesh = MeshInstance3D.new()
	g_mesh.mesh = mesh
	g_mesh.material_override = mat
	g_mesh.position = Vector3(0, height/2.0, 0)
	grass.add_child(g_mesh)

	return grass

func _create_bush() -> Node3D:
	var bush = Node3D.new()
	bush.name = "Bush"

	var radius = rng.randf_range(0.3, 1.0)
	var mesh = SphereMesh.new()
	mesh.radial_segments = 8
	mesh.latitudinal_segments = 6
	mesh.radius = radius
	mesh.height = radius * 1.8

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.15, 0.35, 0.1)
	mat.roughness = 0.9

	var b_mesh = MeshInstance3D.new()
	b_mesh.mesh = mesh
	b_mesh.material_override = mat
	b_mesh.position = Vector3(0, radius * 0.9, 0)
	bush.add_child(b_mesh)

	return bush

func _create_mushroom() -> Node3D:
	var mush = Node3D.new()
	mush.name = "Mushroom"

	var stem_h = rng.randf_range(0.1, 0.25)
	var stem_r = rng.randf_range(0.02, 0.04)
	var cap_r = stem_r * (4.0 + rng.randf() * 3.0)

	var stem_mesh = CylinderMesh.new()
	stem_mesh.top_radius = stem_r * 0.8
	stem_mesh.bottom_radius = stem_r * 1.1
	stem_mesh.height = stem_h
	stem_mesh.radial_segments = 4

	var stem_mat = StandardMaterial3D.new()
	stem_mat.albedo_color = Color(0.7, 0.65, 0.5)
	stem_mat.roughness = 0.9

	var stem = MeshInstance3D.new()
	stem.mesh = stem_mesh
	stem.material_override = stem_mat
	stem.position = Vector3(0, stem_h/2.0, 0)
	mush.add_child(stem)

	var cap_mesh = SphereMesh.new()
	cap_mesh.radial_segments = 8
	cap_mesh.latitudinal_segments = 6
	cap_mesh.radius = cap_r
	cap_mesh.height = cap_r * 0.5

	var cap_mat = StandardMaterial3D.new()
	cap_mat.albedo_color = Color(0.7, 0.2, 0.1)
	cap_mat.roughness = 0.85

	var cap = MeshInstance3D.new()
	cap.mesh = cap_mesh
	cap.material_override = cap_mat
	cap.position = Vector3(0, stem_h, 0)
	mush.add_child(cap)

	return mush

func _create_fern() -> Node3D:
	var fern = Node3D.new()
	fern.name = "Fern"

	var height = rng.randf_range(0.3, 0.6)
	var mesh = CylinderMesh.new()
	mesh.top_radius = 0.01
	mesh.bottom_radius = 0.03
	mesh.height = height
	mesh.radial_segments = 3

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.1, 0.25, 0.05)
	mat.roughness = 0.95

	var f_mesh = MeshInstance3D.new()
	f_mesh.mesh = mesh
	f_mesh.material_override = mat
	f_mesh.position = Vector3(0, height/2.0, 0)
	fern.add_child(f_mesh)

	return fern

func _create_flower() -> Node3D:
	var flower = Node3D.new()
	flower.name = "Flower"

	var height = rng.randf_range(0.2, 0.35)
	var stem_mesh = CylinderMesh.new()
	stem_mesh.top_radius = 0.008
	stem_mesh.bottom_radius = 0.015
	stem_mesh.height = height
	stem_mesh.radial_segments = 3

	var stem_mat = StandardMaterial3D.new()
	stem_mat.albedo_color = Color(0.15, 0.3, 0.05)
	stem_mat.roughness = 0.95

	var stem = MeshInstance3D.new()
	stem.mesh = stem_mesh
	stem.material_override = stem_mat
	stem.position = Vector3(0, height/2.0, 0)
	flower.add_child(stem)

	var head_r = rng.randf_range(0.04, 0.06)
	var head_mesh = SphereMesh.new()
	head_mesh.radial_segments = 6
	head_mesh.latitudinal_segments = 4
	head_mesh.radius = head_r
	head_mesh.height = head_r * 1.2

	var colors = [Color(0.9, 0.2, 0.2), Color(0.8, 0.7, 0.1), Color(0.2, 0.5, 0.9), Color(0.7, 0.2, 0.8)]
	var head_mat = StandardMaterial3D.new()
	head_mat.albedo_color = colors[rng.randi() % colors.size()]
	head_mat.roughness = 0.8

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = head_mat
	head.position = Vector3(0, height, 0)
	flower.add_child(head)

	return flower

func _generate_water():
	for i in range(WATER_BODIES):
		var x = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var z = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var width = rng.randf_range(15.0, 40.0)
		var depth = rng.randf_range(1.0, 3.0)

		var water_body = _create_water_body(width, depth)
		water_body.position = Vector3(x, -depth/2.0, z)
		water.add_child(water_body)

func _create_water_body(width: float, depth: float) -> Node3D:
	var w_body = Node3D.new()
	w_body.name = "Water"

	var mesh = BoxMesh.new()
	mesh.size = Vector3(width, depth * 0.8, width)

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.05, 0.15, 0.3, 0.7)
	mat.metallic = 0.1
	mat.roughness = 0.4
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA

	var w_mesh = MeshInstance3D.new()
	w_mesh.mesh = mesh
	w_mesh.material_override = mat
	w_body.add_child(w_mesh)

	return w_body

func _generate_animals():
	var num_animals = rng.randi_range(10, 30)
	for i in range(num_animals):
		var x = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var z = rng.randf_range(-WORLD_SIZE/2, WORLD_SIZE/2)
		var y = 0

		var animal_type = rng.randi() % 8
		var animal = _create_animal(animal_type)
		animal.position = Vector3(x, y, z)
		animal.rotation = Vector3(0, rng.randf_range(0, TAU), 0)
		animals.add_child(animal)

func _create_animal(animal_type: int) -> Node3D:
	var animal = Node3D.new()

	match animal_type:
		0: animal = _create_deer()
		1: animal = _create_wolf()
		2: animal = _create_bear()
		3: animal = _create_boar()
		4: animal = _create_rabbit()
		5: animal = _create_fox()
		6: animal = _create_rabbit()
		7: animal = _create_deer()

	return animal

func _create_deer() -> Node3D:
	var deer = Node3D.new()
	deer.name = "Deer"

	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.8, 0.5, 1.2)

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.5, 0.35, 0.2)
	body_mat.roughness = 0.85

	var body = MeshInstance3D.new()
	body.mesh = body_mesh
	body.material_override = body_mat
	body.position = Vector3(0, 0.4, 0)
	deer.add_child(body)

	var leg_h = 0.4
	var leg_r = 0.05
	var leg_positions = [Vector3(-0.35, leg_h/2.0, 0.4), Vector3(0.35, leg_h/2.0, 0.4), Vector3(-0.35, leg_h/2.0, -0.4), Vector3(0.35, leg_h/2.0, -0.4)]

	for pos in leg_positions:
		var leg_mesh = CylinderMesh.new()
		leg_mesh.top_radius = leg_r
		leg_mesh.bottom_radius = leg_r
		leg_mesh.height = leg_h
		leg_mesh.radial_segments = 4

		var leg = MeshInstance3D.new()
		leg.mesh = leg_mesh
		leg.material_override = body_mat
		leg.position = pos
		deer.add_child(leg)

	var neck_mesh = CylinderMesh.new()
	neck_mesh.top_radius = 0.04
	neck_mesh.bottom_radius = 0.05
	neck_mesh.height = 0.5
	neck_mesh.radial_segments = 4

	var neck = MeshInstance3D.new()
	neck.mesh = neck_mesh
	neck.material_override = body_mat
	neck.position = Vector3(0, 0.55, 0.5)
	neck.rotation.x = -0.5
	deer.add_child(neck)

	var head_mesh = BoxMesh.new()
	head_mesh.size = Vector3(0.15, 0.15, 0.25)

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = body_mat
	head.position = Vector3(0, 0.8, 0.7)
	deer.add_child(head)

	return deer

func _create_wolf() -> Node3D:
	var wolf = Node3D.new()
	wolf.name = "Wolf"

	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.5, 0.4, 0.9)

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.25, 0.25, 0.3)
	body_mat.roughness = 0.85

	var body = MeshInstance3D.new()
	body.mesh = body_mesh
	body.material_override = body_mat
	body.position = Vector3(0, 0.3, 0)
	wolf.add_child(body)

	var leg_h = 0.3
	var leg_r = 0.04
	var leg_positions = [Vector3(-0.2, leg_h/2.0, 0.3), Vector3(0.2, leg_h/2.0, 0.3), Vector3(-0.2, leg_h/2.0, -0.3), Vector3(0.2, leg_h/2.0, -0.3)]

	for pos in leg_positions:
		var leg_mesh = CylinderMesh.new()
		leg_mesh.top_radius = leg_r
		leg_mesh.bottom_radius = leg_r
		leg_mesh.height = leg_h
		leg_mesh.radial_segments = 4

		var leg = MeshInstance3D.new()
		leg.mesh = leg_mesh
		leg.material_override = body_mat
		leg.position = pos
		wolf.add_child(leg)

	var head_mesh = BoxMesh.new()
	head_mesh.size = Vector3(0.2, 0.18, 0.3)

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = body_mat
	head.position = Vector3(0, 0.4, 0.6)
	wolf.add_child(head)

	return wolf

func _create_bear() -> Node3D:
	var bear = Node3D.new()
	bear.name = "Bear"

	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(1.2, 0.9, 1.5)

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.2, 0.15, 0.1)
	body_mat.roughness = 0.9

	var body = MeshInstance3D.new()
	body.mesh = body_mesh
	body.material_override = body_mat
	body.position = Vector3(0, 0.55, 0)
	bear.add_child(body)

	var leg_h = 0.5
	var leg_r = 0.12
	var leg_positions = [Vector3(-0.45, leg_h/2.0, 0.5), Vector3(0.45, leg_h/2.0, 0.5), Vector3(-0.45, leg_h/2.0, -0.5), Vector3(0.45, leg_h/2.0, -0.5)]

	for pos in leg_positions:
		var leg_mesh = CylinderMesh.new()
		leg_mesh.top_radius = leg_r
		leg_mesh.bottom_radius = leg_r
		leg_mesh.height = leg_h
		leg_mesh.radial_segments = 4

		var leg = MeshInstance3D.new()
		leg.mesh = leg_mesh
		leg.material_override = body_mat
		leg.position = pos
		bear.add_child(leg)

	var head_mesh = BoxMesh.new()
	head_mesh.size = Vector3(0.35, 0.25, 0.4)

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = body_mat
	head.position = Vector3(0, 0.75, 0.7)
	bear.add_child(head)

	return bear

func _create_boar() -> Node3D:
	var boar = Node3D.new()
	boar.name = "Boar"

	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.7, 0.5, 1.1)

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.15, 0.1, 0.05)
	body_mat.roughness = 0.9

	var body = MeshInstance3D.new()
	body.mesh = body_mesh
	body.material_override = body_mat
	body.position = Vector3(0, 0.35, 0)
	boar.add_child(body)

	var leg_h = 0.3
	var leg_r = 0.06
	var leg_positions = [Vector3(-0.25, leg_h/2.0, 0.4), Vector3(0.25, leg_h/2.0, 0.4), Vector3(-0.25, leg_h/2.0, -0.4), Vector3(0.25, leg_h/2.0, -0.4)]

	for pos in leg_positions:
		var leg_mesh = CylinderMesh.new()
		leg_mesh.top_radius = leg_r
		leg_mesh.bottom_radius = leg_r
		leg_mesh.height = leg_h
		leg_mesh.radial_segments = 4

		var leg = MeshInstance3D.new()
		leg.mesh = leg_mesh
		leg.material_override = body_mat
		leg.position = pos
		boar.add_child(leg)

	var head_mesh = BoxMesh.new()
	head_mesh.size = Vector3(0.25, 0.2, 0.35)

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = body_mat
	head.position = Vector3(0, 0.45, 0.55)
	boar.add_child(head)

	return boar

func _create_rabbit() -> Node3D:
	var rabbit = Node3D.new()
	rabbit.name = "Rabbit"

	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.15, 0.12, 0.25)

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.6, 0.55, 0.4)
	body_mat.roughness = 0.9

	var body = MeshInstance3D.new()
	body.mesh = body_mesh
	body.material_override = body_mat
	body.position = Vector3(0, 0.1, 0)
	rabbit.add_child(body)

	var head_mesh = BoxMesh.new()
	head_mesh.size = Vector3(0.1, 0.1, 0.12)

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = body_mat
	head.position = Vector3(0, 0.15, 0.18)
	rabbit.add_child(head)

	var ear_mesh = BoxMesh.new()
	ear_mesh.size = Vector3(0.03, 0.08, 0.02)

	var ear_l = MeshInstance3D.new()
	ear_l.mesh = ear_mesh
	ear_l.material_override = body_mat
	ear_l.position = Vector3(-0.04, 0.22, 0.18)
	rabbit.add_child(ear_l)

	var ear_r = MeshInstance3D.new()
	ear_r.mesh = ear_mesh
	ear_r.material_override = body_mat
	ear_r.position = Vector3(0.04, 0.22, 0.18)
	rabbit.add_child(ear_r)

	return rabbit

func _create_fox() -> Node3D:
	var fox = Node3D.new()
	fox.name = "Fox"

	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.4, 0.35, 0.7)

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.8, 0.4, 0.1)
	body_mat.roughness = 0.85

	var body = MeshInstance3D.new()
	body.mesh = body_mesh
	body.material_override = body_mat
	body.position = Vector3(0, 0.25, 0)
	fox.add_child(body)

	var leg_h = 0.25
	var leg_r = 0.035
	var leg_positions = [Vector3(-0.15, leg_h/2.0, 0.25), Vector3(0.15, leg_h/2.0, 0.25), Vector3(-0.15, leg_h/2.0, -0.25), Vector3(0.15, leg_h/2.0, -0.25)]

	for pos in leg_positions:
		var leg_mesh = CylinderMesh.new()
		leg_mesh.top_radius = leg_r
		leg_mesh.bottom_radius = leg_r
		leg_mesh.height = leg_h
		leg_mesh.radial_segments = 4

		var leg = MeshInstance3D.new()
		leg.mesh = leg_mesh
		leg.material_override = body_mat
		leg.position = pos
		fox.add_child(leg)

	var head_mesh = BoxMesh.new()
	head_mesh.size = Vector3(0.15, 0.12, 0.2)

	var head = MeshInstance3D.new()
	head.mesh = head_mesh
	head.material_override = body_mat
	head.position = Vector3(0, 0.32, 0.4)
	fox.add_child(head)

	return fox