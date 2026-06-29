extends Node3D
class_name EntitySpawner

var _schema_loader: SchemaLoader
var _meta_builder: MetaEntityBuilder
var _spawned_entities: Dictionary = {}

signal entity_spawned(entity_id: String, node_path: NodePath)
signal spawn_failed(reason: String)

const SPAWN_CONFIG = {
	"building": {
		"collision_shape": "BoxShape3D",
		"scale_range": [Vector3(3, 4, 3), Vector3(8, 12, 8)],
		"y_offset": 0.0,
		"lod_distances": [30.0, 60.0, 120.0],
	},
	"tree": {
		"collision_shape": "CylinderShape3D",
		"scale_range": [Vector3(0.8, 3, 0.8), Vector3(1.5, 8, 1.5)],
		"y_offset": 0.0,
		"lod_distances": [20.0, 50.0, 100.0],
	},
	"rock": {
		"collision_shape": "SphereShape3D",
		"scale_range": [Vector3(0.3, 0.3, 0.3), Vector3(2.0, 2.0, 2.0)],
		"y_offset": 0.0,
		"lod_distances": [15.0, 40.0, 80.0],
	},
	"npc": {
		"collision_shape": "CapsuleShape3D",
		"scale_range": [Vector3(1, 1, 1), Vector3(1, 1, 1)],
		"y_offset": 0.0,
		"lod_distances": [25.0, 60.0, 120.0],
	},
	"vehicle": {
		"collision_shape": "BoxShape3D",
		"scale_range": [Vector3(2, 1.5, 4), Vector3(3, 2, 6)],
		"y_offset": 0.5,
		"lod_distances": [40.0, 80.0, 150.0],
	},
	"animal": {
		"collision_shape": "CapsuleShape3D",
		"scale_range": [Vector3(0.5, 0.5, 0.5), Vector3(1.5, 1.5, 1.5)],
		"y_offset": 0.3,
		"lod_distances": [20.0, 50.0, 100.0],
	},
}

func setup(schema_loader: SchemaLoader, meta_builder: MetaEntityBuilder):
	_schema_loader = schema_loader
	_meta_builder = meta_builder

func spawn_from_glb(glb_path: String, position: Vector3, entity_type: String = "building") -> Node3D:
	var entity_node = Node3D.new()
	entity_node.name = glb_path.get_file().get_basename()

	var model = _load_glb(glb_path)
	if model == null:
		spawn_failed.emit("Failed to load GLB: " + glb_path)
		entity_node.queue_free()
		return null

	entity_node.add_child(model)

	var entity_id = ""
	if _meta_builder:
		entity_id = _meta_builder.build_from_glb(glb_path, position)

	_add_collision(entity_node, entity_type)
	_add_interaction(entity_node, entity_type, entity_id)

	if entity_id != "":
		_spawned_entities[entity_id] = entity_node

	entity_node.position = position
	add_child(entity_node)
	entity_spawned.emit(entity_id, entity_node.get_path())

	return entity_node

func spawn_from_schema(schema_name: String, position: Vector3, entity_type: String = "building") -> Node3D:
	var entity_node = Node3D.new()
	entity_node.name = schema_name

	var entity_id = ""
	if _meta_builder:
		entity_id = _meta_builder.build_from_schema(schema_name, position)

	_create_visual_from_schema(entity_node, schema_name)
	_add_collision(entity_node, entity_type)
	_add_interaction(entity_node, entity_type, entity_id)

	if entity_id != "":
		_spawned_entities[entity_id] = entity_node

	entity_node.position = position
	add_child(entity_node)
	entity_spawned.emit(entity_id, entity_node.get_path())

	return entity_node

func spawn_procedural(entity_type: String, position: Vector3, variant: int = 0) -> Node3D:
	var entity_node = Node3D.new()
	entity_node.name = entity_type + "_" + str(variant)

	match entity_type:
		"building":
			_create_procedural_building(entity_node, variant)
		"tree":
			_create_procedural_tree(entity_node, variant)
		"rock":
			_create_procedural_rock(entity_node, variant)
		"npc":
			_create_procedural_npc(entity_node, variant)
		"animal":
			_create_procedural_animal(entity_node, variant)
		_:
			_create_procedural_generic(entity_node, variant)

	_add_collision(entity_node, entity_type)

	var entity_id = ""
	if _meta_builder:
		var material = _get_material_for_type(entity_type)
		if _meta_builder._meta_node and _meta_builder._meta_node.has_method("spawn_entity"):
			entity_id = _meta_builder._meta_node.spawn_entity(material, position.x, position.y, position.z)

	if entity_id != "":
		_spawned_entities[entity_id] = entity_node

	entity_node.position = position
	add_child(entity_node)
	entity_spawned.emit(entity_id, entity_node.get_path())

	return entity_node

func populate_area(center: Vector3, radius: float, density: Dictionary = {}) -> Array:
	var nodes: Array = []

	var defaults = {
		"building": 5,
		"tree": 30,
		"rock": 15,
		"npc": 3,
		"animal": 8,
	}
	var merged = defaults.merge(density)

	for entity_type in merged:
		var count = merged[entity_type]
		for i in range(count):
			var angle = randf() * TAU
			var dist = randf() * radius
			var x = center.x + cos(angle) * dist
			var z = center.z + sin(angle) * dist
			var pos = Vector3(x, 0, z)

			var node = spawn_procedural(entity_type, pos, i)
			if node:
				nodes.append(node)

	return nodes

func _load_glb(path: String) -> Node3D:
	if not FileAccess.file_exists(path):
		return null

	var scene = load(path)
	if scene == null:
		return null

	var instance = scene.instantiate()
	return instance

func _add_collision(node: Node3D, entity_type: String):
	var config = SPAWN_CONFIG.get(entity_type, SPAWN_CONFIG["rock"])

	var body = StaticBody3D.new()
	body.name = "CollisionBody"

	var shape = CollisionShape3D.new()
	shape.name = "CollisionShape"

	var collision = _create_collision_shape(entity_type, config)
	shape.shape = collision

	body.add_child(shape)
	node.add_child(body)

func _create_collision_shape(entity_type: String, config: Dictionary) -> Shape3D:
	var shape_type = config.get("collision_shape", "BoxShape3D")

	match shape_type:
		"BoxShape3D":
			var s = BoxShape3D.new()
			var scale_range = config.get("scale_range", [Vector3.ONE, Vector3.ONE * 2])
			var size = scale_range[0].lerp(scale_range[1], 0.5)
			s.size = size
			return s
		"CylinderShape3D":
			var s = CylinderShape3D.new()
			s.radius = 0.5
			s.height = 2.0
			return s
		"SphereShape3D":
			var s = SphereShape3D.new()
			s.radius = 0.5
			return s
		"CapsuleShape3D":
			var s = CapsuleShape3D.new()
			s.radius = 0.3
			s.height = 1.6
			return s
		_:
			var s = BoxShape3D.new()
			s.size = Vector3.ONE
			return s

func _add_interaction(node: Node3D, entity_type: String, entity_id: String):
	var interaction = Node.new()
	interaction.name = "InteractionData"
	interaction.set_meta("entity_type", entity_type)
	interaction.set_meta("entity_id", entity_id)
	interaction.set_meta("interactable", true)

	match entity_type:
		"building":
			interaction.set_meta("interaction_type", "enter")
			interaction.set_meta("damageable", true)
		"tree":
			interaction.set_meta("interaction_type", "harvest")
			interaction.set_meta("damageable", true)
			interaction.set_meta("flammable", true)
		"rock":
			interaction.set_meta("interaction_type", "mine")
			interaction.set_meta("damageable", true)
		"npc":
			interaction.set_meta("interaction_type", "talk")
			interaction.set_meta("damageable", true)
		"animal":
			interaction.set_meta("interaction_type", "hunt")
			interaction.set_meta("damageable", true)
		"vehicle":
			interaction.set_meta("interaction_type", "drive")
		_:
			interaction.set_meta("interaction_type", "examine")

	node.add_child(interaction)

func _create_visual_from_schema(node: Node3D, schema_name: String):
	if not _schema_loader:
		return

	var schema = _schema_loader.get_schema(schema_name)
	if schema.is_empty():
		return

	var parts = schema.get("parts", [])
	for part_data in parts:
		var part_name = part_data.get("name", "part")
		var mesh_instance = MeshInstance3D.new()
		mesh_instance.name = part_name

		var box = BoxMesh.new()
		mesh_instance.mesh = box

		var mat = StandardMaterial3D.new()
		var material_label = part_data.get("material_label", "generic")
		mat.albedo_color = _material_to_color(material_label)
		mesh_instance.material_override = mat

		node.add_child(mesh_instance)

func _create_procedural_building(node: Node3D, variant: int):
	var width = randf_range(4, 10)
	var height = randf_range(5, 15)
	var depth = randf_range(4, 10)

	var body = MeshInstance3D.new()
	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(width, height, depth)
	body.mesh = body_mesh

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.5, 0.48, 0.45)
	body.material_override = mat
	body.position = Vector3(0, height * 0.5, 0)
	node.add_child(body)

	if randf() > 0.3:
		var roof = MeshInstance3D.new()
		var roof_mesh = BoxMesh.new()
		roof_mesh.size = Vector3(width + 0.5, 0.3, depth + 0.5)
		roof.mesh = roof_mesh
		var roof_mat = StandardMaterial3D.new()
		roof_mat.albedo_color = Color(0.35, 0.25, 0.2)
		roof.material_override = roof_mat
		roof.position = Vector3(0, height + 0.15, 0)
		node.add_child(roof)

	var window_count = int(width * depth * 0.3)
	for i in range(window_count):
		var win = MeshInstance3D.new()
		var win_mesh = BoxMesh.new()
		win_mesh.size = Vector3(0.8, 1.0, 0.05)
		win.mesh = win_mesh
		var win_mat = StandardMaterial3D.new()
		win_mat.albedo_color = Color(0.3, 0.4, 0.5)
		win_mat.emission_enabled = randf() > 0.7
		if win_mat.emission_enabled:
			win_mat.emission = Color(0.8, 0.7, 0.3)
		win.material_override = win_mat

		var side = randi() % 4
		var wy = randf_range(2, height - 1)
		match side:
			0:
				win.position = Vector3(randf_range(-width*0.4, width*0.4), wy, depth*0.5)
			1:
				win.position = Vector3(randf_range(-width*0.4, width*0.4), wy, -depth*0.5)
				win.rotation.y = PI
			2:
				win.position = Vector3(width*0.5, wy, randf_range(-depth*0.4, depth*0.4))
				win.rotation.y = PI/2
			3:
				win.position = Vector3(-width*0.5, wy, randf_range(-depth*0.4, depth*0.4))
				win.rotation.y = -PI/2

		node.add_child(win)

func _create_procedural_tree(node: Node3D, variant: int):
	var height = randf_range(4, 10)
	var trunk_radius = randf_range(0.2, 0.4)

	var trunk = MeshInstance3D.new()
	var trunk_mesh = CylinderMesh.new()
	trunk_mesh.height = height * 0.7
	trunk_mesh.top_radius = trunk_radius * 0.7
	trunk_mesh.bottom_radius = trunk_radius
	trunk.mesh = trunk_mesh

	var trunk_mat = StandardMaterial3D.new()
	trunk_mat.albedo_color = Color(0.45, 0.35, 0.25)
	trunk.material_override = trunk_mat
	trunk.position = Vector3(0, height * 0.35, 0)
	node.add_child(trunk)

	var foliage = MeshInstance3D.new()
	var foliage_mesh = SphereMesh.new()
	foliage_mesh.radius = randf_range(1.5, 3.0)
	foliage.mesh = foliage_mesh

	var foliage_mat = StandardMaterial3D.new()
	foliage_mat.albedo_color = Color(0.2 + randf()*0.15, 0.45 + randf()*0.2, 0.15 + randf()*0.1)
	foliage.material_override = foliage_mat
	foliage.position = Vector3(0, height * 0.85, 0)
	node.add_child(foliage)

func _create_procedural_rock(node: Node3D, variant: int):
	var size = randf_range(0.3, 2.0)

	var rock = MeshInstance3D.new()
	var rock_mesh = BoxMesh.new()
	rock_mesh.size = Vector3(size * 1.2, size * 0.5, size * 1.2)
	rock.mesh = rock_mesh

	var rock_mat = StandardMaterial3D.new()
	rock_mat.albedo_color = Color(0.5 + randf()*0.1, 0.5 + randf()*0.1, 0.5 + randf()*0.1)
	rock.material_override = rock_mat
	rock.position = Vector3(0, size * 0.25, 0)
	rock.rotation = Vector3(randf()*0.3, randf()*TAU, randf()*0.3)
	node.add_child(rock)

func _create_procedural_npc(node: Node3D, variant: int):
	var body = MeshInstance3D.new()
	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.4, 1.2, 0.25)
	body.mesh = body_mesh

	var body_mat = StandardMaterial3D.new()
	var colors = [Color(0.5, 0.4, 0.3), Color(0.3, 0.3, 0.5), Color(0.4, 0.5, 0.3), Color(0.6, 0.3, 0.3)]
	body_mat.albedo_color = colors[variant % colors.size()]
	body.material_override = body_mat
	body.position = Vector3(0, 0.8, 0)
	node.add_child(body)

	var head = MeshInstance3D.new()
	var head_mesh = SphereMesh.new()
	head_mesh.radius = 0.2
	head.mesh = head_mesh

	var head_mat = StandardMaterial3D.new()
	head_mat.albedo_color = Color(0.9, 0.8, 0.7)
	head.material_override = head_mat
	head.position = Vector3(0, 1.6, 0)
	node.add_child(head)

func _create_procedural_animal(node: Node3D, variant: int):
	var types = ["deer", "wolf", "rabbit", "fox"]
	var type = types[variant % types.size()]

	var size = 0.5
	var color = Color.GRAY

	match type:
		"deer":
			size = 1.0
			color = Color(0.75, 0.55, 0.35)
		"wolf":
			size = 0.7
			color = Color(0.45, 0.45, 0.45)
		"rabbit":
			size = 0.3
			color = Color(0.9, 0.9, 0.9)
		"fox":
			size = 0.5
			color = Color(1.0, 0.55, 0.25)

	var body = MeshInstance3D.new()
	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(size * 0.5, size * 0.7, size * 0.3)
	body.mesh = body_mesh

	var mat = StandardMaterial3D.new()
	mat.albedo_color = color
	body.material_override = mat
	body.position = Vector3(0, size * 0.35, 0)
	node.add_child(body)

func _create_procedural_generic(node: Node3D, variant: int):
	var mesh_instance = MeshInstance3D.new()
	var mesh = BoxMesh.new()
	mesh.size = Vector3.ONE
	mesh_instance.mesh = mesh

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(randf(), randf(), randf())
	mesh_instance.material_override = mat
	mesh_instance.position = Vector3(0, 0.5, 0)
	node.add_child(mesh_instance)

func _material_to_color(material: String) -> Color:
	var colors = {
		"metal_steel": Color(0.6, 0.6, 0.65),
		"metal_iron": Color(0.5, 0.45, 0.4),
		"concrete": Color(0.5, 0.48, 0.45),
		"wood_oak": Color(0.45, 0.35, 0.25),
		"wood_pine": Color(0.6, 0.45, 0.3),
		"glass": Color(0.7, 0.8, 0.9),
		"vegetation_leaf": Color(0.25, 0.55, 0.2),
		"vegetation_bark": Color(0.4, 0.3, 0.2),
		"flesh": Color(0.9, 0.8, 0.7),
		"rubber": Color(0.15, 0.15, 0.15),
		"soil": Color(0.4, 0.3, 0.2),
		"water": Color(0.2, 0.4, 0.8),
		"sand": Color(0.85, 0.75, 0.55),
		"stone_granite": Color(0.55, 0.55, 0.55),
	}
	return colors.get(material, Color(0.7, 0.7, 0.7))

func _get_material_for_type(entity_type: String) -> String:
	match entity_type:
		"building":
			return "concrete"
		"tree":
			return "wood_oak"
		"rock":
			return "stone_granite"
		"npc":
			return "flesh"
		"animal":
			return "flesh"
		"vehicle":
			return "metal_steel"
		_:
			return "generic"

func get_spawned_entity(entity_id: String) -> Node3D:
	if _spawned_entities.has(entity_id):
		return _spawned_entities[entity_id]
	return null

func get_all_spawned_ids() -> Array:
	return _spawned_entities.keys()

func despawn_entity(entity_id: String) -> bool:
	if not _spawned_entities.has(entity_id):
		return false

	var node = _spawned_entities[entity_id]
	_spawned_entities.erase(entity_id)
	node.queue_free()
	return true

func clear_all():
	for id in _spawned_entities:
		var node = _spawned_entities[id]
		if is_instance_valid(node):
			node.queue_free()
	_spawned_entities.clear()
