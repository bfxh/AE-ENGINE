extends Node3D
class_name WorldManager

var schema_loader: SchemaLoader
var meta_builder: MetaEntityBuilder
var entity_spawner: EntitySpawner
var meta_node: Node

var _initialized: bool = false
var _tick: int = 0

signal world_ready()
signal world_tick(tick: int)
signal world_error(msg: String)

func _ready():
	_init_subsystems()

func _init_subsystems():
	schema_loader = SchemaLoader.new()
	schema_loader.name = "SchemaLoader"
	add_child(schema_loader)

	meta_builder = MetaEntityBuilder.new()
	meta_builder.name = "MetaEntityBuilder"
	add_child(meta_builder)

	entity_spawner = EntitySpawner.new()
	entity_spawner.name = "EntitySpawner"
	add_child(entity_spawner)

	_connect_meta_node()

	meta_builder.setup(schema_loader, meta_node)
	entity_spawner.setup(schema_loader, meta_builder)

	_initialized = true
	world_ready.emit()
	print("[WorldManager] Subsystems initialized")

func _connect_meta_node():
	var candidates = [
		"WastelandMeta",
		"WastelandWorld",
		"MetaNode",
	]

	for child in get_children():
		for candidate in candidates:
			if child.name == candidate:
				meta_node = child
				print("[WorldManager] Connected meta node: ", child.name)
				return

	var parent_children = get_parent().get_children() if get_parent() else []
	for child in parent_children:
		for candidate in candidates:
			if child.name == candidate:
				meta_node = child
				print("[WorldManager] Connected meta node from parent: ", child.name)
				return

func _process(delta):
	if not _initialized:
		return

	_tick += 1
	if _tick % 60 == 0:
		world_tick.emit(_tick)

func load_schema_directory(path: String) -> int:
	var count = schema_loader.load_all_manifests(path)
	print("[WorldManager] Loaded %d schemas from %s" % [count, path])
	return count

func spawn_entity_from_glb(glb_path: String, position: Vector3, entity_type: String = "building") -> Node3D:
	return entity_spawner.spawn_from_glb(glb_path, position, entity_type)

func spawn_entity_from_schema(schema_name: String, position: Vector3, entity_type: String = "building") -> Node3D:
	return entity_spawner.spawn_from_schema(schema_name, position, entity_type)

func spawn_procedural(entity_type: String, position: Vector3, variant: int = 0) -> Node3D:
	return entity_spawner.spawn_procedural(entity_type, position, variant)

func populate_area(center: Vector3, radius: float, density: Dictionary = {}) -> Array:
	return entity_spawner.populate_area(center, radius, density)

func generate_wasteland_scene(radius: float = 100.0) -> Dictionary:
	var density = {
		"building": 8,
		"tree": 40,
		"rock": 20,
		"npc": 4,
		"animal": 10,
	}

	var nodes = populate_area(Vector3.ZERO, radius, density)

	var result = {
		"total_entities": nodes.size(),
		"buildings": 0,
		"trees": 0,
		"rocks": 0,
		"npcs": 0,
		"animals": 0,
	}

	for node in nodes:
		if node.name.begins_with("building"):
			result["buildings"] += 1
		elif node.name.begins_with("tree"):
			result["trees"] += 1
		elif node.name.begins_with("rock"):
			result["rocks"] += 1
		elif node.name.begins_with("npc"):
			result["npcs"] += 1
		elif node.name.begins_with("animal"):
			result["animals"] += 1

	print("[WorldManager] Generated wasteland: ", result)
	return result

func get_entity_properties(entity_id: String) -> Dictionary:
	return meta_builder.get_entity_properties(entity_id)

func apply_damage_to_entity(entity_id: String, amount: float):
	meta_builder.apply_damage(entity_id, amount)

func apply_heat_to_entity(entity_id: String, delta: float):
	meta_builder.apply_heat(entity_id, delta)

func trigger_reaction(entity_a: String, entity_b: String) -> Dictionary:
	return meta_builder.trigger_reaction(entity_a, entity_b)

func get_world_stats() -> Dictionary:
	var stats = {
		"tick": _tick,
		"schema_count": schema_loader.get_loaded_schema_names().size(),
		"spawned_entities": entity_spawner.get_all_spawned_ids().size(),
		"initialized": _initialized,
	}

	if meta_node:
		if meta_node.has_method("entity_count"):
			stats["meta_entity_count"] = meta_node.entity_count()
		if meta_node.has_method("get_stats"):
			stats["world_stats"] = meta_node.get_stats()

	return stats

func clear_world():
	entity_spawner.clear_all()
	schema_loader.clear()
	print("[WorldManager] World cleared")
