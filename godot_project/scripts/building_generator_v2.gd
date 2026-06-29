extends Node
class_name BuildingGenerator

enum BuildingType {
	RUINED_HOUSE, COLLAPSED_TOWER, BUNKER_ENTRANCE, SCRAP_WALL,
	WATCHTOWER, BRIDGE_REMNANT, FACTORY_RUIN,
}

var building_materials = {}
var rng: RandomNumberGenerator

func _init():
	rng = RandomNumberGenerator.new()
	_setup_materials()

func _setup_materials():
	building_materials["concrete"] = _make_material(Color(0.45, 0.43, 0.4), 0.9)
	building_materials["brick"] = _make_material(Color(0.55, 0.25, 0.15), 0.85)
	building_materials["steel"] = _make_material(Color(0.3, 0.2, 0.15), 0.5, 0.3)
	building_materials["wood"] = _make_material(Color(0.25, 0.15, 0.05), 0.7)
	building_materials["rubble"] = _make_material(Color(0.4, 0.38, 0.35), 0.95)

func _make_material(color: Color, roughness: float, metallic: float = 0.0) -> StandardMaterial3D:
	var mat = StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = roughness
	mat.metallic = metallic
	return mat

func generate_ruined_house(position: Vector3, p_rng: RandomNumberGenerator):
	rng = p_rng
	var house = Node3D.new()
	house.name = "RuinedHouse"
	house.position = position

	var floors = rng.randi_range(1, 3)
	var width = rng.randf_range(6.0, 12.0)
	var depth = rng.randf_range(5.0, 10.0)

	for floor in range(floors):
		var floor_y = floor * 3.0

		var wall_thickness = 0.3
		var height = rng.randf_range(2.5, 3.5)

		var walls = _create_wall_segments(width, depth, height, wall_thickness, floor_y)

		for wall_data in walls:
			var wall_mesh = MeshInstance3D.new()
			wall_mesh.mesh = BoxMesh.new()
			wall_mesh.mesh.size = wall_data["size"]

			var wall_node = Node3D.new()
			wall_node.position = wall_data["position"]
			wall_node.rotation_degrees = wall_data["rotation"]
			wall_node.add_child(wall_mesh)

			if rng.randf() < 0.3:
				wall_mesh.position.y += rng.randf_range(-0.3, 0.5)
				wall_node.rotation_degrees.x += rng.randf_range(-5, 5)
				wall_node.rotation_degrees.z += rng.randf_range(-5, 5)

			if rng.randf() < 0.4:
				var gap = rng.randf_range(0.5, 2.0)
				wall_mesh.mesh.size.x -= gap

			var mat = building_materials["concrete"] if rng.randf() < 0.6 else building_materials["brick"]
			wall_mesh.material_override = mat

			house.add_child(wall_node)

		if floor == 0:
			var floor_mesh = MeshInstance3D.new()
			floor_mesh.mesh = BoxMesh.new()
			floor_mesh.mesh.size = Vector3(width, 0.2, depth)
			floor_mesh.position.y = -0.1
			floor_mesh.material_override = building_materials["concrete"]
			house.add_child(floor_mesh)

	var roof_height = floors * 3.0 + rng.randf_range(1.0, 2.5)
	var roof_count = rng.randi_range(0, 3)
	for i in range(roof_count):
		var roof_mesh = MeshInstance3D.new()
		roof_mesh.mesh = BoxMesh.new()
		roof_mesh.mesh.size = Vector3(rng.randf_range(1.0, 3.0), 0.15, rng.randf_range(1.0, 2.0))
		roof_mesh.position = Vector3(rng.randf_range(-3, 3), roof_height, rng.randf_range(-2, 2))
		roof_mesh.rotation_degrees = Vector3(rng.randf_range(0, 20), 0, rng.randf_range(0, 10))
		roof_mesh.material_override = building_materials["wood"]
		house.add_child(roof_mesh)

	add_rubble_around(house, Vector3.ZERO, width, depth)
	return house

func generate_collapsed_tower(position: Vector3, p_rng: RandomNumberGenerator):
	rng = p_rng
	var tower = Node3D.new()
	tower.name = "CollapsedTower"
	tower.position = position

	var base_radius = rng.randf_range(2.0, 4.0)
	var total_height = rng.randf_range(8.0, 20.0)
	var segments = rng.randi_range(3, 6)

	for seg in range(segments):
		var seg_y = seg * (total_height / segments)
		var seg_height = total_height / segments
		var radius = base_radius * (1.0 - seg * 0.1)

		for side in range(rng.randi_range(3, 6)):
			var angle = side * TAU / 6.0 + rng.randf_range(-0.2, 0.2)
			var wall_width = rng.randf_range(0.5, 1.5)
			var x = cos(angle) * (radius - wall_width / 2)
			var z = sin(angle) * (radius - wall_width / 2)

			var wall_mesh = MeshInstance3D.new()
			wall_mesh.mesh = BoxMesh.new()
			wall_mesh.mesh.size = Vector3(wall_width, seg_height, 0.3)

			var wall_node = Node3D.new()
			wall_node.position = Vector3(x, seg_y, z)
			wall_node.rotation_degrees.y = rad_to_deg(angle) + 90

			if seg > segments * 0.6:
				if rng.randf() < 0.4:
					wall_node.rotation_degrees.x += rng.randf_range(-30, 30)
					wall_node.rotation_degrees.z += rng.randf_range(-20, 20)
					wall_node.position.y += rng.randf_range(-2, 2)

			wall_mesh.material_override = building_materials["concrete"] if rng.randf() < 0.5 else building_materials["steel"]
			wall_node.add_child(wall_mesh)
			tower.add_child(wall_node)

	var top_debris = rng.randi_range(3, 10)
	for i in range(top_debris):
		var debris = MeshInstance3D.new()
		debris.mesh = BoxMesh.new()
		debris.mesh.size = Vector3(rng.randf_range(0.3, 1.5), rng.randf_range(0.1, 0.5), rng.randf_range(0.3, 1.0))
		debris.position = Vector3(rng.randf_range(-4, 4), total_height + rng.randf_range(-1, 3), rng.randf_range(-4, 4))
		debris.rotation_degrees = Vector3(rng.randf_range(0, 90), rng.randf_range(0, 360), rng.randf_range(0, 90))
		debris.material_override = building_materials["rubble"]
		tower.add_child(debris)

	add_rubble_around(tower, Vector3.ZERO, base_radius * 2, base_radius * 2)
	return tower

func generate_bunker_entrance(position: Vector3, p_rng: RandomNumberGenerator):
	rng = p_rng
	var bunker = Node3D.new()
	bunker.name = "BunkerEntrance"
	bunker.position = position

	var entrance_width = rng.randf_range(2.0, 3.5)
	var entrance_height = rng.randf_range(2.0, 3.0)
	var door_depth = rng.randf_range(0.5, 1.5)

	var frame_mat = building_materials["steel"] if rng.randf() < 0.5 else building_materials["concrete"]

	var top = MeshInstance3D.new()
	top.mesh = BoxMesh.new()
	top.mesh.size = Vector3(entrance_width + 0.6, 0.4, door_depth + 0.6)
	top.position.y = entrance_height / 2
	top.material_override = frame_mat
	bunker.add_child(top)

	var left = MeshInstance3D.new()
	left.mesh = BoxMesh.new()
	left.mesh.size = Vector3(0.3, entrance_height, door_depth)
	left.position = Vector3(-entrance_width / 2 - 0.15, 0, 0)
	left.material_override = frame_mat
	bunker.add_child(left)

	var right = MeshInstance3D.new()
	right.mesh = BoxMesh.new()
	right.mesh.size = Vector3(0.3, entrance_height, door_depth)
	right.position = Vector3(entrance_width / 2 + 0.15, 0, 0)
	right.material_override = frame_mat
	bunker.add_child(right)

	var ground_hill = MeshInstance3D.new()
	ground_hill.mesh = BoxMesh.new()
	ground_hill.mesh.size = Vector3(entrance_width + 3.0, 1.0, door_depth + 4.0)
	ground_hill.position = Vector3(0, -0.8, door_depth / 2 + 1.0)
	ground_hill.material_override = building_materials["rubble"]
	bunker.add_child(ground_hill)

	if rng.randf() < 0.5:
		var door = MeshInstance3D.new()
		door.mesh = BoxMesh.new()
		door.mesh.size = Vector3(entrance_width - 0.2, entrance_height - 0.2, 0.1)
		door.position.z = door_depth / 2
		door.rotation_degrees.x = rng.randf_range(10, 40)
		door.material_override = building_materials["steel"]
		bunker.add_child(door)

	return bunker

func generate_scrap_wall(position: Vector3, p_rng: RandomNumberGenerator):
	rng = p_rng
	var wall = Node3D.new()
	wall.name = "ScrapWall"
	wall.position = position

	var wall_length = rng.randf_range(4.0, 15.0)
	var wall_height = rng.randf_range(1.5, 3.5)
	var segment_count = int(wall_length / 1.5)

	for seg in range(segment_count):
		var seg_width = rng.randf_range(1.0, 2.5)
		var seg_height = rng.randf_range(wall_height * 0.5, wall_height)
		var x_pos = seg * 1.5 - wall_length / 2

		var seg_mesh = MeshInstance3D.new()
		seg_mesh.mesh = BoxMesh.new()
		seg_mesh.mesh.size = Vector3(seg_width, seg_height, rng.randf_range(0.15, 0.4))
		seg_mesh.position = Vector3(x_pos, seg_height / 2, rng.randf_range(-0.2, 0.2))
		seg_mesh.rotation_degrees = Vector3(0, rng.randf_range(-10, 10), rng.randf_range(-5, 5))

		var mat = building_materials["steel"] if rng.randf() < 0.4 else building_materials["wood"]
		seg_mesh.material_override = mat
		wall.add_child(seg_mesh)

	return wall

func _create_wall_segments(width: float, depth: float, height: float, thickness: float, floor_y: float) -> Array:
	var segments = []
	var hw = width / 2
	var hd = depth / 2
	var hy = height / 2 + floor_y

	segments.append({
		"position": Vector3(0, hy, -hd),
		"size": Vector3(width, height, thickness),
		"rotation": Vector3.ZERO
	})
	segments.append({
		"position": Vector3(0, hy, hd),
		"size": Vector3(width, height, thickness),
		"rotation": Vector3.ZERO
	})
	segments.append({
		"position": Vector3(-hw, hy, 0),
		"size": Vector3(depth, height, thickness),
		"rotation": Vector3(0, 90, 0)
	})
	segments.append({
		"position": Vector3(hw, hy, 0),
		"size": Vector3(depth, height, thickness),
		"rotation": Vector3(0, 90, 0)
	})

	return segments

func add_rubble_around(parent: Node3D, center: Vector3, width: float, depth: float):
	var rubble_count = rng.randi_range(5, 20)
	for i in range(rubble_count):
		var rubble = MeshInstance3D.new()
		rubble.mesh = BoxMesh.new()
		rubble.mesh.size = Vector3(
			rng.randf_range(0.2, 1.5),
			rng.randf_range(0.1, 0.6),
			rng.randf_range(0.2, 1.0)
		)
		rubble.position = Vector3(
			center.x + rng.randf_range(-width / 2 - 2, width / 2 + 2),
			0.1,
			center.z + rng.randf_range(-depth / 2 - 2, depth / 2 + 2)
		)
		rubble.rotation_degrees = Vector3(
			rng.randf_range(0, 90),
			rng.randf_range(0, 360),
			rng.randf_range(0, 90)
		)
		rubble.material_override = building_materials["rubble"] if rng.randf() < 0.7 else building_materials["brick"]
		parent.add_child(rubble)