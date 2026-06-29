extends Node
class_name SchemaLoader

var _loaded_schemas: Dictionary = {}
var _part_registry: Dictionary = {}

signal schema_loaded(schema_name: String)
signal part_registered(part_name: String, schema: Dictionary)
signal schema_error(schema_name: String, error: String)

const SEMANTIC_TYPES = {
	"torso": "body_core",
	"head": "body_core",
	"arm_left": "body_limb",
	"arm_right": "body_limb",
	"leg_left": "body_limb",
	"leg_right": "body_limb",
	"hand_left": "body_extremity",
	"hand_right": "body_extremity",
	"foot_left": "body_extremity",
	"foot_right": "body_extremity",
	"wall": "structure_vertical",
	"floor": "structure_horizontal",
	"roof": "structure_top",
	"window": "structure_opening",
	"door": "structure_opening",
	"pillar": "structure_support",
	"beam": "structure_support",
	"trunk": "organic_core",
	"branch": "organic_limb",
	"foliage": "organic_mass",
	"root": "organic_base",
}

const JOINT_DEFAULTS = {
	"revolute": {"axis": Vector3(0, 1, 0), "min_angle": -90.0, "max_angle": 90.0},
	"prismatic": {"axis": Vector3(0, 1, 0), "min_dist": 0.0, "max_dist": 1.0},
	"spherical": {"min_angle": -180.0, "max_angle": 180.0},
	"fixed": {},
	"hinge": {"axis": Vector3(0, 1, 0), "min_angle": -120.0, "max_angle": 120.0},
}

func load_manifest(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		schema_error.emit(path, "File not found: " + path)
		return {}

	var file = FileAccess.open(path, FileAccess.READ)
	if file == null:
		schema_error.emit(path, "Cannot open file: " + path)
		return {}

	var json_text = file.get_as_text()
	file.close()

	var json = JSON.new()
	var err = json.parse(json_text)
	if err != OK:
		schema_error.emit(path, "JSON parse error: " + json.get_error_message())
		return {}

	var data = json.data
	if not data is Dictionary:
		schema_error.emit(path, "Root must be a Dictionary")
		return {}

	var schema_name = data.get("schema_name", path.get_file().get_basename())
	_loaded_schemas[schema_name] = data

	_register_parts(schema_name, data)

	schema_loaded.emit(schema_name)
	return data

func _register_parts(schema_name: String, data: Dictionary):
	var parts = data.get("parts", [])
	for part in parts:
		var part_name = part.get("name", "unknown")
		var enriched = _enrich_part(part)
		var key = schema_name + "/" + part_name
		_part_registry[key] = enriched
		part_registered.emit(part_name, enriched)

func _enrich_part(part: Dictionary) -> Dictionary:
	var enriched = part.duplicate(true)

	if not enriched.has("semantic_type"):
		enriched["semantic_type"] = _infer_semantic_type(part.get("name", ""))

	if not enriched.has("joint_type"):
		enriched["joint_type"] = _infer_joint_type(enriched["semantic_type"])

	if not enriched.has("material_label"):
		enriched["material_label"] = _infer_material(enriched["semantic_type"])

	if not enriched.has("interaction_tags"):
		enriched["interaction_tags"] = _infer_interaction_tags(enriched["semantic_type"])

	if not enriched.has("joint_config"):
		var jt = enriched["joint_type"]
		if JOINT_DEFAULTS.has(jt):
			enriched["joint_config"] = JOINT_DEFAULTS[jt].duplicate(true)
		else:
			enriched["joint_config"] = {}

	if not enriched.has("physics_properties"):
		enriched["physics_properties"] = _get_physics_defaults(enriched["material_label"])

	return enriched

func _infer_semantic_type(part_name: String) -> String:
	var lower = part_name.to_lower()
	for keyword in SEMANTIC_TYPES:
		if lower.find(keyword) != -1:
			return SEMANTIC_TYPES[keyword]
	return "generic"

func _infer_joint_type(semantic_type: String) -> String:
	match semantic_type:
		"body_limb":
			return "revolute"
		"body_extremity":
			return "spherical"
		"body_core":
			return "fixed"
		"structure_vertical", "structure_horizontal", "structure_top":
			return "fixed"
		"structure_opening":
			return "hinge"
		"structure_support":
			return "fixed"
		"organic_limb":
			return "revolute"
		"organic_core", "organic_mass", "organic_base":
			return "fixed"
		_:
			return "fixed"

func _infer_material(semantic_type: String) -> String:
	match semantic_type:
		"body_core", "body_limb", "body_extremity":
			return "flesh"
		"structure_vertical", "structure_horizontal", "structure_top":
			return "concrete"
		"structure_opening":
			return "metal_steel"
		"structure_support":
			return "metal_steel"
		"organic_core", "organic_limb":
			return "wood_oak"
		"organic_mass":
			return "vegetation_leaf"
		"organic_base":
			return "soil"
		_:
			return "generic"

func _infer_interaction_tags(semantic_type: String) -> Array:
	match semantic_type:
		"body_core", "body_limb", "body_extremity":
			return ["damageable", "grabbable"]
		"structure_vertical", "structure_horizontal", "structure_top":
			return ["damageable", "climbable"]
		"structure_opening":
			return ["openable", "damageable"]
		"structure_support":
			return ["damageable"]
		"organic_core":
			return ["damageable", "harvestable"]
		"organic_limb":
			return ["damageable", "breakable"]
		"organic_mass":
			return ["harvestable", "flammable"]
		"organic_base":
			return ["damageable"]
		_:
			return ["damageable"]

func _get_physics_defaults(material: String) -> Dictionary:
	var defaults = {
		"generic": {"density": 1000.0, "friction": 0.5, "restitution": 0.3},
		"flesh": {"density": 1050.0, "friction": 0.6, "restitution": 0.1},
		"metal_steel": {"density": 7800.0, "friction": 0.4, "restitution": 0.2},
		"concrete": {"density": 2400.0, "friction": 0.7, "restitution": 0.1},
		"wood_oak": {"density": 600.0, "friction": 0.5, "restitution": 0.2},
		"vegetation_leaf": {"density": 300.0, "friction": 0.4, "restitution": 0.3},
		"soil": {"density": 1500.0, "friction": 0.8, "restitution": 0.05},
		"glass": {"density": 2500.0, "friction": 0.3, "restitution": 0.1},
		"rubber": {"density": 1100.0, "friction": 0.9, "restitution": 0.8},
	}
	return defaults.get(material, defaults["generic"]).duplicate(true)

func get_schema(name: String) -> Dictionary:
	return _loaded_schemas.get(name, {})

func get_part(schema_name: String, part_name: String) -> Dictionary:
	var key = schema_name + "/" + part_name
	return _part_registry.get(key, {})

func get_all_parts(schema_name: String) -> Array:
	var result = []
	for key in _part_registry:
		if key.begins_with(schema_name + "/"):
			result.append(_part_registry[key])
	return result

func get_parts_by_semantic_type(semantic_type: String) -> Array:
	var result = []
	for key in _part_registry:
		var part = _part_registry[key]
		if part.get("semantic_type", "") == semantic_type:
			result.append(part)
	return result

func get_parts_by_material(material_label: String) -> Array:
	var result = []
	for key in _part_registry:
		var part = _part_registry[key]
		if part.get("material_label", "") == material_label:
			result.append(part)
	return result

func get_loaded_schema_names() -> Array:
	return _loaded_schemas.keys()

func load_all_manifests(directory: String) -> int:
	var count = 0
	var dir = DirAccess.open(directory)
	if dir == null:
		return 0

	dir.list_dir_begin()
	var file_name = dir.get_next()
	while file_name != "":
		if file_name.ends_with("_manifest.json"):
			var full_path = directory.path_join(file_name)
			var result = load_manifest(full_path)
			if not result.is_empty():
				count += 1
		file_name = dir.get_next()
	dir.list_dir_end()

	return count

func clear():
	_loaded_schemas.clear()
	_part_registry.clear()
