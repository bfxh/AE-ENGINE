extends Node
class_name MetaEntityBuilder

var _schema_loader: SchemaLoader
var _meta_node: Node

signal entity_built(entity_id: String, schema_name: String)
signal build_error(entity_name: String, error: String)

func setup(schema_loader: SchemaLoader, meta_node: Node):
	_schema_loader = schema_loader
	_meta_node = meta_node

func build_from_glb(glb_path: String, position: Vector3) -> String:
	var manifest_path = glb_path.get_basename() + "_manifest.json"
	if not FileAccess.file_exists(manifest_path):
		build_error.emit(glb_path, "No manifest found for: " + glb_path)
		return ""

	var data = _schema_loader.load_manifest(manifest_path)
	if data.is_empty():
		build_error.emit(glb_path, "Failed to load manifest")
		return ""

	var schema_name = data.get("schema_name", "")
	var material = _infer_primary_material(data)

	var entity_id = ""
	if _meta_node and _meta_node.has_method("spawn_entity"):
		entity_id = _meta_node.spawn_entity(material, position.x, position.y, position.z)
	else:
		entity_id = _generate_fallback_id()

	_register_parts_to_entity(entity_id, data)

	entity_built.emit(entity_id, schema_name)
	return entity_id

func build_from_schema(schema_name: String, position: Vector3) -> String:
	var data = _schema_loader.get_schema(schema_name)
	if data.is_empty():
		build_error.emit(schema_name, "Schema not found: " + schema_name)
		return ""

	var material = _infer_primary_material(data)

	var entity_id = ""
	if _meta_node and _meta_node.has_method("spawn_entity"):
		entity_id = _meta_node.spawn_entity(material, position.x, position.y, position.z)
	else:
		entity_id = _generate_fallback_id()

	_register_parts_to_entity(entity_id, data)

	entity_built.emit(entity_id, schema_name)
	return entity_id

func _register_parts_to_entity(entity_id: String, data: Dictionary):
	var parts = data.get("parts", [])
	for part_data in parts:
		var part_name = part_data.get("name", "unknown")
		var semantic_type = part_data.get("semantic_type", "generic")
		var material_label = part_data.get("material_label", "generic")
		var joint_type = part_data.get("joint_type", "fixed")
		var interaction_tags = part_data.get("interaction_tags", [])

		if _meta_node and _meta_node.has_method("apply_heat_to_entity"):
			var physics_props = part_data.get("physics_properties", {})
			var density = physics_props.get("density", 1000.0)
			if _meta_node.has_method("set_part_property"):
				_meta_node.set_part_property(entity_id, part_name, "density", density)

func _infer_primary_material(data: Dictionary) -> String:
	var parts = data.get("parts", [])
	if parts.is_empty():
		return "generic"

	var material_counts: Dictionary = {}
	for part in parts:
		var mat = part.get("material_label", "generic")
		material_counts[mat] = material_counts.get(mat, 0) + 1

	var best_material = "generic"
	var best_count = 0
	for mat in material_counts:
		if material_counts[mat] > best_count:
			best_count = material_counts[mat]
			best_material = mat

	return best_material

func _generate_fallback_id() -> String:
	return "entity_%d_%d" % [Time.get_ticks_msec(), randi() % 10000]

func batch_build(directory: String, center: Vector3, spacing: float = 5.0) -> Array:
	var ids = []
	var dir = DirAccess.open(directory)
	if dir == null:
		return ids

	var glb_files: Array = []
	dir.list_dir_begin()
	var file_name = dir.get_next()
	while file_name != "":
		if file_name.ends_with(".glb"):
			glb_files.append(file_name)
		file_name = dir.get_next()
	dir.list_dir_end()

	var count = glb_files.size()
	var cols = ceili(sqrt(count))
	var row = 0
	var col = 0

	for glb_file in glb_files:
		var offset = Vector3(col * spacing, 0, row * spacing)
		var pos = center + offset
		var id = build_from_glb(directory.path_join(glb_file), pos)
		if id != "":
			ids.append(id)

		col += 1
		if col >= cols:
			col = 0
			row += 1

	return ids

func get_entity_properties(entity_id: String) -> Dictionary:
	if _meta_node and _meta_node.has_method("get_entity_properties"):
		return _meta_node.get_entity_properties(entity_id)
	return {}

func apply_damage(entity_id: String, amount: float):
	if _meta_node and _meta_node.has_method("apply_damage_to_entity"):
		_meta_node.apply_damage_to_entity(entity_id, amount)

func apply_heat(entity_id: String, delta: float):
	if _meta_node and _meta_node.has_method("apply_heat_to_entity"):
		_meta_node.apply_heat_to_entity(entity_id, delta)

func trigger_reaction(entity_a: String, entity_b: String) -> Dictionary:
	if _meta_node and _meta_node.has_method("trigger_reaction_between"):
		return _meta_node.trigger_reaction_between(entity_a, entity_b)
	return {}
