extends Node3D
class_name VegetationSpawner

var vegetation_parent: Node3D
var spawn_seed: int = 999
var spawn_radius: float = 230.0
var vegetation_count: int = 2000

var rock_count: int = 300
var grass_patch_count: int = 800
var bush_count: int = 400
var fern_count: int = 300
var flower_count: int = 200
var fallen_log_count: int = 50
var mushroom_count: int = 150

func _ready():
	vegetation_parent = Node3D.new()
	vegetation_parent.name = "Vegetation"
	add_child(vegetation_parent)
	spawn_all()

func spawn_all():
	var rng = RandomNumberGenerator.new()
	rng.seed = spawn_seed

	spawn_rocks(rng)
	spawn_grass_patches(rng)
	spawn_bushes(rng)
	spawn_ferns(rng)
	spawn_flowers(rng)
	spawn_fallen_logs(rng)
	spawn_mushrooms(rng)

	print("[Vegetation] Spawned: %d rocks, %d grass, %d bushes, %d ferns, %d flowers, %d logs, %d mushrooms" % [
		rock_count, grass_patch_count, bush_count, fern_count, flower_count, fallen_log_count, mushroom_count
	])

func _get_ground_height(x: float, z: float) -> float:
	var space_state = get_world_3d().direct_space_state
	if space_state:
		var ray_query = PhysicsRayQueryParameters3D.create(Vector3(x, 500, z), Vector3(x, -50, z))
		var result = space_state.intersect_ray(ray_query)
		if result:
			return result.position.y
	return -999.0

func _random_position(rng: RandomNumberGenerator, min_dist: float, avoid_steep: bool = true):
	for attempt in range(8):
		var angle = rng.randf() * TAU
		var dist = rng.randf_range(min_dist, spawn_radius)
		var x = cos(angle) * dist
		var z = sin(angle) * dist

		if avoid_steep:
			var h_center = _get_ground_height(x, z)
			if h_center < -20:
				continue
			var h_off1 = _get_ground_height(x + 1, z)
			var h_off2 = _get_ground_height(x, z + 1)
			if abs(h_off1 - h_center) > 2 or abs(h_off2 - h_center) > 2:
				continue
			return Vector3(x, h_center, z)
		else:
			var h = _get_ground_height(x, z)
			if h > -20:
				return Vector3(x, h, z)
	return null

func spawn_rocks(rng: RandomNumberGenerator):
	var rock_mat = StandardMaterial3D.new()
	for i in range(rock_count):
		var pos = _random_position(rng, 10.0, false)
		if pos == null:
			continue

		var rock = MeshInstance3D.new()
		var size = rng.randf_range(0.3, 3.0)
		rock.scale = Vector3(size, size * rng.randf_range(0.4, 0.8), size * rng.randf_range(0.6, 1.0))

		if rng.randf() < 0.5:
			rock.mesh = SphereMesh.new()
		else:
			rock.mesh = BoxMesh.new()
			rock.mesh.size = Vector3(1.2, 0.8, 1.0)

		rock_mat.albedo_color = Color(
			0.4 + rng.randf_range(-0.1, 0.1),
			0.35 + rng.randf_range(-0.1, 0.1),
			0.3 + rng.randf_range(-0.1, 0.1)
		)
		rock_mat.roughness = 0.7
		rock_mat.metallic = 0.05
		rock.material_override = rock_mat.duplicate()

		rock.position = pos + Vector3(0, size * 0.3, 0)
		rock.rotation = Vector3(rng.randf() * 0.3, rng.randf() * TAU, rng.randf() * 0.3)
		rock.create_trimesh_collision()

		vegetation_parent.add_child(rock)

func spawn_grass_patches(rng: RandomNumberGenerator):
	for i in range(grass_patch_count):
		var pos = _random_position(rng, 10.0)
		if pos == null:
			continue

		var patch = Node3D.new()
		patch.name = "GrassPatch"

		var blade_count = rng.randi_range(3, 12)
		for b in range(blade_count):
			var blade = MeshInstance3D.new()
			var bmesh = CylinderMesh.new()
			bmesh.top_radius = 0.0
			bmesh.bottom_radius = 0.03
			var h = rng.randf_range(0.3, 1.2)
			bmesh.height = h

			var mat = StandardMaterial3D.new()
			mat.albedo_color = Color(
				0.15 + rng.randf_range(0.0, 0.2),
				0.3 + rng.randf_range(0.0, 0.25),
				0.05 + rng.randf_range(0.0, 0.1)
			)
			mat.roughness = 0.9
			blade.material_override = mat

			blade.position = Vector3(rng.randf_range(-0.5, 0.5), h / 2, rng.randf_range(-0.5, 0.5))
			blade.rotation = Vector3(rng.randf_range(-0.2, 0.2), rng.randf() * TAU, rng.randf_range(-0.2, 0.2))
			patch.add_child(blade)

		patch.position = pos
		vegetation_parent.add_child(patch)

func spawn_bushes(rng: RandomNumberGenerator):
	var bush_mat = StandardMaterial3D.new()
	for i in range(bush_count):
		var pos = _random_position(rng, 15.0)
		if pos == null:
			continue

		var bush = MeshInstance3D.new()
		var radius = rng.randf_range(0.4, 1.5)
		bush.mesh = SphereMesh.new()
		bush.mesh.radius = radius
		bush.mesh.height = radius * 2
		bush.mesh.radial_segments = 8

		bush_mat.albedo_color = Color(
			0.08 + rng.randf_range(0.0, 0.1),
			0.25 + rng.randf_range(0.0, 0.2),
			0.08 + rng.randf_range(0.0, 0.08)
		)
		bush_mat.roughness = 0.8
		bush.material_override = bush_mat.duplicate()

		bush.position = pos + Vector3(0, radius * 0.4, 0)
		bush.scale = Vector3(1.0, rng.randf_range(0.6, 0.9), 1.0)
		vegetation_parent.add_child(bush)

func spawn_ferns(rng: RandomNumberGenerator):
	for i in range(fern_count):
		var pos = _random_position(rng, 10.0)
		if pos == null:
			continue

		var fern = Node3D.new()
		fern.name = "Fern"

		var frond_count = rng.randi_range(3, 8)
		for f in range(frond_count):
			var frond = MeshInstance3D.new()
			var fmesh = BoxMesh.new()
			fmesh.size = Vector3(0.8, 0.02, 0.15)

			var mat = StandardMaterial3D.new()
			mat.albedo_color = Color(0.1, 0.4 + rng.randf_range(0.0, 0.2), 0.1)
			mat.roughness = 0.8
			frond.material_override = mat

			var angle = float(f) / float(frond_count) * TAU
			frond.position = Vector3(cos(angle) * 0.3, 0, sin(angle) * 0.3)
			frond.rotation_degrees = Vector3(rng.randf_range(-30, 30), rad_to_deg(angle), rng.randf_range(-15, 15))
			fern.add_child(frond)

		fern.position = pos
		fern.scale = Vector3.ONE * rng.randf_range(0.5, 1.5)
		vegetation_parent.add_child(fern)

func spawn_flowers(rng: RandomNumberGenerator):
	for i in range(flower_count):
		var pos = _random_position(rng, 10.0)
		if pos == null:
			continue

		var flower = Node3D.new()
		flower.name = "Flower"

		var stem = MeshInstance3D.new()
		var smesh = CylinderMesh.new()
		smesh.top_radius = 0.01
		smesh.bottom_radius = 0.02
		smesh.height = rng.randf_range(0.2, 0.6)
		stem.mesh = smesh
		var stem_mat = StandardMaterial3D.new()
		stem_mat.albedo_color = Color(0.1, 0.4, 0.1)
		stem.material_override = stem_mat
		stem.position.y = smesh.height / 2
		flower.add_child(stem)

		var head = MeshInstance3D.new()
		head.mesh = SphereMesh.new()
		head.mesh.radius = 0.06
		head.mesh.height = 0.12

		var head_mat = StandardMaterial3D.new()
		var hue = rng.randf()
		if hue < 0.3:
			head_mat.albedo_color = Color(1, 0.8, 0.1)
		elif hue < 0.5:
			head_mat.albedo_color = Color(1, 0.2, 0.3)
		elif hue < 0.7:
			head_mat.albedo_color = Color(0.8, 0.2, 1)
		elif hue < 0.85:
			head_mat.albedo_color = Color(1, 0.5, 0.8)
		else:
			head_mat.albedo_color = Color(0.3, 0.6, 1)

		head.material_override = head_mat
		head.position.y = smesh.height + 0.05
		flower.add_child(head)

		flower.position = pos
		vegetation_parent.add_child(flower)

func spawn_fallen_logs(rng: RandomNumberGenerator):
	var log_mat = StandardMaterial3D.new()
	for i in range(fallen_log_count):
		var pos = _random_position(rng, 20.0, false)
		if pos == null:
			continue

		var log = MeshInstance3D.new()
		log.mesh = CylinderMesh.new()
		log.mesh.top_radius = 0.3
		log.mesh.bottom_radius = 0.4
		log.mesh.height = rng.randf_range(2.0, 6.0)

		log_mat.albedo_color = Color(0.3, 0.2, 0.1).darkened(rng.randf_range(0.0, 0.3))
		log_mat.roughness = 0.9
		log.material_override = log_mat.duplicate()

		log.position = pos + Vector3(0, 0.15, 0)
		log.rotation_degrees = Vector3(90 + rng.randf_range(-10, 10), rng.randf() * 360, 0)
		log.create_trimesh_collision()
		vegetation_parent.add_child(log)

func spawn_mushrooms(rng: RandomNumberGenerator):
	for i in range(mushroom_count):
		var pos = _random_position(rng, 15.0)
		if pos == null:
			continue

		var mushroom = Node3D.new()
		mushroom.name = "Mushroom"

		var stalk = MeshInstance3D.new()
		stalk.mesh = CylinderMesh.new()
		stalk.mesh.top_radius = 0.03
		stalk.mesh.bottom_radius = 0.04
		var stalk_h = rng.randf_range(0.05, 0.2)
		stalk.mesh.height = stalk_h
		var stalk_mat = StandardMaterial3D.new()
		stalk_mat.albedo_color = Color(0.7, 0.65, 0.55)
		stalk.material_override = stalk_mat
		stalk.position.y = stalk_h / 2
		mushroom.add_child(stalk)

		var cap = MeshInstance3D.new()
		cap.mesh = SphereMesh.new()
		cap.mesh.radius = rng.randf_range(0.05, 0.15)
		cap.mesh.height = cap.mesh.radius * 2

		var cap_mat = StandardMaterial3D.new()
		var cap_hue = rng.randf()
		if cap_hue < 0.3:
			cap_mat.albedo_color = Color(0.8, 0.2, 0.1)
		elif cap_hue < 0.5:
			cap_mat.albedo_color = Color(0.7, 0.5, 0.2)
		elif cap_hue < 0.7:
			cap_mat.albedo_color = Color(0.9, 0.8, 0.6)
		else:
			cap_mat.albedo_color = Color(0.2, 0.7, 0.3)

		cap.material_override = cap_mat
		cap.position.y = stalk_h
		cap.scale = Vector3(1, 0.5, 1)
		mushroom.add_child(cap)

		mushroom.position = pos + Vector3(rng.randf_range(-0.3, 0.3), 0, rng.randf_range(-0.3, 0.3))
		vegetation_parent.add_child(mushroom)