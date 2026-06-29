extends Node

const SPEC_NAME = "Wasteland Resource Compatibility Standard"
const SPEC_VERSION = "1.0.0"

static func validate_model_format(file_path: String) -> Dictionary:
	var ext = file_path.get_extension().to_lower()
	var allowed = ["glb", "gltf", "fbx", "obj", "blend"]
	return {
		"valid": ext in allowed,
		"format": ext,
		"preferred": ext in ["glb", "gltf"],
	}

static func validate_texture_format(file_path: String) -> Dictionary:
	var ext = file_path.get_extension().to_lower()
	var allowed = ["png", "jpg", "jpeg", "webp", "tga", "bmp", "hdr", "exr"]
	return {
		"valid": ext in allowed,
		"format": ext,
		"preferred": ext in ["png", "webp", "jpg"],
	}

static func validate_naming(file_name: String, category: String) -> Dictionary:
	var warnings = []

	if " " in file_name:
		warnings.append("Contains spaces - use underscores")
	if any(c.isupper() for c in file_name.replace('.', '')):
		warnings.append("Contains uppercase - use lowercase")
	if file_name.count('.') > 1:
		warnings.append("Multiple dots in filename")

	var prefixes = {
		"tree": ["tree_", "t_"],
		"rock": ["rock_", "r_", "stone_"],
		"building": ["bld_", "building_", "ruin_"],
		"character": ["char_", "npc_", "chr_"],
		"animal": ["anim_", "animal_", "creature_"],
		"prop": ["prop_", "item_", "p_"],
	}

	if category in prefixes:
		var has_prefix = false
		for prefix in prefixes[category]:
			if file_name.to_lower().begins_with(prefix):
				has_prefix = true
				break
		if not has_prefix:
			warnings.append("No standard prefix for '%s'" % category)

	return {
		"valid": len(warnings) == 0,
		"warnings": warnings,
	}

static func validate_triangle_count(mesh_path: String, category: String) -> Dictionary:
	var limits = {
		"tree": {"min": 50, "max": 5000, "recommended": 2000},
		"rock": {"min": 20, "max": 3000, "recommended": 500},
		"building": {"min": 100, "max": 20000, "recommended": 5000},
		"character": {"min": 1000, "max": 30000, "recommended": 13000},
		"animal": {"min": 500, "max": 15000, "recommended": 3000},
		"prop": {"min": 10, "max": 5000, "recommended": 500},
	}

	var default_limit = {"min": 10, "max": 50000, "recommended": 3000}
	var limit = limits.get(category, default_limit)

	return {
		"limits": limit,
		"note": "Verify in Blender or Godot after import",
	}

static func validate_material_setup(material_data: Dictionary) -> Dictionary:
	var issues = []
	var required_maps = ["albedo", "normal", "roughness"]

	if not material_data.get("has_albedo", false):
		issues.append("Missing albedo/diffuse map")
	if not material_data.get("has_normal", false):
		issues.append("Missing normal map (recommended)")
	if not material_data.get("has_pbr", false):
		issues.append("Not using PBR material")

	return {
		"valid": len(issues) <= 1,
		"issues": issues,
	}

static func lua_compatibility_check(asset_name: String, asset_type: String) -> Dictionary:
	var checks = {
		"format": validate_model_format(asset_name),
		"naming": validate_naming(asset_name, asset_type),
		"tris": validate_triangle_count(asset_name, asset_type),
	}

	var all_valid = true
	var issues = []

	for check_name in checks:
		var result = checks[check_name]
		if not result.get("valid", false):
			all_valid = false
			if result.has("warnings"):
				issues.append_array(result["warnings"])
			elif result.has("issues"):
				issues.append_array(result["issues"])

	return {
		"asset": asset_name,
		"type": asset_type,
		"compatible": all_valid,
		"issues": issues,
		"checks": checks,
	}

static func get_required_lod_levels(category: String) -> Array:
	var lod_configs = {
		"tree": [{"distance": 0, "ratio": 1.0}, {"distance": 30, "ratio": 0.5}, {"distance": 80, "ratio": 0.1}],
		"rock": [{"distance": 0, "ratio": 1.0}, {"distance": 40, "ratio": 0.3}],
		"building": [{"distance": 0, "ratio": 1.0}, {"distance": 50, "ratio": 0.5}, {"distance": 100, "ratio": 0.1}],
		"character": [{"distance": 0, "ratio": 1.0}, {"distance": 20, "ratio": 0.5}, {"distance": 60, "ratio": 0.1}],
		"animal": [{"distance": 0, "ratio": 1.0}, {"distance": 30, "ratio": 0.5}, {"distance": 80, "ratio": 0.1}],
	}

	var default_lod = [{"distance": 0, "ratio": 1.0}]
	return lod_configs.get(category, default_lod)

static func get_import_settings(format: String) -> Dictionary:
	return {
		"glb": {
			"generate_physics": false,
			"import_animations": true,
			"import_skins": true,
			"import_blend_shapes": true,
			"compress_meshes": true,
			"use_legacy_gltf2": false,
		},
		"gltf": {
			"generate_physics": false,
			"import_animations": true,
			"import_skins": true,
			"compress_meshes": true,
		},
	}

static func get_pbr_material_config() -> Dictionary:
	return {
		"metallic": 0.0,
		"roughness": 0.7,
		"specular": 0.5,
		"albedo_color": Color.WHITE,
		"uv1_scale": Vector3(1, 1, 1),
		"emission_enabled": false,
		"transparency": BaseMaterial3D.TRANSPARENCY_DISABLED,
	}

static func print_compatibility_report(assets: Array) -> String:
	var report = "============================================================" + "\n"
	report += "  RESOURCE COMPATIBILITY REPORT\n"
	report += "  Standard: %s v%s\n" % [SPEC_NAME, SPEC_VERSION]
	report += "============================================================" + "\n"

	var compatible = 0
	var incompatible = 0

	for asset in assets:
		var result = lua_compatibility_check(asset.get("name", "unknown"), asset.get("type", "prop"))
		var status = "[OK]" if result["compatible"] else "[!!]"
		report += "\n%s %s (%s)\n" % [status, asset.get("name", "unknown"), asset.get("type", "prop")]
		if result["compatible"]:
			compatible += 1
		else:
			incompatible += 1
			for issue in result["issues"]:
				report += "  - %s\n" % issue

	report += "\n============================================================" + "\n"
	report += "  Compatible: %d, Incompatible: %d\n" % [compatible, incompatible]
	report += "============================================================"

	return report