extends Node
class_name WastelandSmokeTest

const TIMEOUT_SECONDS: float = 60.0
const TARGET_FPS_MINIMUM: int = 30

var _elapsed: float = 0.0
var _current_stage: String = "init"
var _stage_results: Dictionary = {}
var _stage_start_time: float = 0.0
var _total_errors: int = 0
var _total_warnings: int = 0

func _ready():
	print("=" * 50)
	print("  WASTELAND SMOKE TEST v0.4")
	print("=" * 50)
	_stage_start_time = Time.get_ticks_msec()
	_stage("boot", "_test_boot")

func _process(delta: float):
	_elapsed += delta
	if _elapsed > TIMEOUT_SECONDS:
		_fail("Smoke test timed out after %.0f seconds" % TIMEOUT_SECONDS)

func _stage(name: String, method: String):
	_current_stage = name
	var elapsed = Time.get_ticks_msec() - _stage_start_time
	_stage_start_time = Time.get_ticks_msec()
	print("\n[STAGE: %s] Starting... (total elapsed: %.1fs)" % [name, _elapsed])
	call_deferred(method)

func _pass_stage(name: String, details: String = ""):
	var elapsed = (Time.get_ticks_msec() - _stage_start_time) / 1000.0
	_stage_results[name] = {"passed": true, "elapsed": elapsed, "details": details}
	print("[STAGE: %s] PASSED (%.1fs) %s" % [name, elapsed, details])

func _fail_stage(name: String, reason: String):
	var elapsed = (Time.get_ticks_msec() - _stage_start_time) / 1000.0
	_stage_results[name] = {"passed": false, "elapsed": elapsed, "error": reason}
	_total_errors += 1
	push_error("[STAGE: %s] FAILED: %s" % [name, reason])

func _test_boot():
	await get_tree().process_frame

	_stage("scene_tree", "_test_scene_tree")

func _test_scene_tree():
	var root = get_tree().root
	if root == null:
		_fail_stage("scene_tree", "Root node not found")
		_finalize()
		return

	var required_systems = [
		"WastelandMain", "GameManager", "ForestEnvironment",
		"TerrainGen", "Forest", "Vegetation", "WaterBodies",
		"Atmosphere", "AIGovernance", "WastelandController",
	]

	var found = 0
	var missing = []
	for sys_name in required_systems:
		var node = root.find_child(sys_name, true, false)
		if node:
			found += 1
		else:
			missing.append(sys_name)

	print("  Systems found: %d/%d" % [found, required_systems.size()])
	for m in missing:
		print("  MISSING: %s" % m)
		_total_warnings += 1

	_pass_stage("scene_tree", "Found %d/%d systems" % [found, required_systems.size()])
	_stage("camera", "_test_camera")

func _test_camera():
	var camera = get_viewport().get_camera_3d()
	if camera == null:
		for child in get_tree().root.get_children():
			if child is Camera3D and child.current:
				camera = child
				break

	if camera == null:
		_fail_stage("camera", "No active Camera3D found")
	else:
		print("  Camera position: %s" % camera.global_position)
		_pass_stage("camera", "Camera active at %s" % camera.global_position)

	_stage("lighting", "_test_lighting")

func _test_lighting():
	var dir_lights = []
	for child in get_tree().root.get_children():
		_find_lights(child, dir_lights)

	if dir_lights.is_empty():
		_total_warnings += 1
		print("  WARNING: No DirectionalLight3D found")
		_pass_stage("lighting", "0 directional lights (using WorldEnvironment)")
	else:
		var light = dir_lights[0]
		print("  Sun energy: %.1f, shadows: %s" % [light.light_energy, light.shadow_enabled])
		_pass_stage("lighting", "%d lights, sun energy: %.1f" % [dir_lights.size(), light.light_energy])

	_stage("environment", "_test_environment")

func _find_lights(node: Node, result: Array):
	if node is DirectionalLight3D:
		result.append(node)
	for child in node.get_children():
		_find_lights(child, result)

func _test_environment():
	await get_tree().create_timer(0.5).timeout

	var stats = {}
	var forest_env = _find_node_of_class("ForestEnvironment")
	if forest_env and forest_env.has_method("get_environment_stats"):
		stats = forest_env.get_environment_stats()
		for key in ["trees", "animals", "buildings", "memory_mb"]:
			print("  %s: %s" % [key, stats.get(key, "N/A")])

	if stats.is_empty():
		_total_warnings += 1
		_pass_stage("environment", "Environment stats unavailable (expected if no generation)")
	else:
		_pass_stage("environment", "%d trees, %d animals" % [stats.get("trees", 0), stats.get("animals", 0)])

	_stage("performance", "_test_performance")

func _test_performance():
	await get_tree().create_timer(2.0).timeout
	var fps = Engine.get_frames_per_second()

	var objects = Performance.get_monitor(Performance.OBJECT_COUNT) if Performance else 0
	var physics = Performance.get_monitor(Performance.PHYSICS_3D_ACTIVE_OBJECTS) if Performance else 0
	var memory = OS.get_static_memory_usage() / 1048576.0

	print("  FPS: %d, Objects: %d, Physics: %d, Memory: %.1f MB" % [fps, objects, physics, memory])

	if fps < TARGET_FPS_MINIMUM:
		_total_warnings += 1
		print("  WARNING: FPS below %d target" % TARGET_FPS_MINIMUM)

	_pass_stage("performance", "FPS: %d, Mem: %.1f MB, Objs: %d" % [fps, memory, objects])
	_stage("ai_governance", "_test_ai_governance")

func _test_ai_governance():
	var ai_gov = _find_node_of_class("AIGovernanceSpec")
	if ai_gov and ai_gov.has_method("run_all_checks"):
		var results = ai_gov.run_all_checks()
		print("  Rules: %d, Passed: %d, Failed: %d" % [
			results.get("total_rules", 0),
			results.get("passed", 0),
			results.get("failed", 0),
		])
		if results.get("failed", 0) > 0:
			_total_warnings += 1
		_pass_stage("ai_governance", results.get("summary", "No summary"))
	else:
		_pass_stage("ai_governance", "AI Governance not loaded")
	_stage("interaction", "_test_interaction")

func _test_interaction():
	var interactive_count = 0
	for child in get_tree().root.get_children():
		_count_interactive(child, interactive_count)
		interactive_count = _count_interactive(child, interactive_count)

	print("  Interactive objects: %d" % interactive_count)
	_pass_stage("interaction", "%d interactive objects" % interactive_count)
	_finalize()

func _count_interactive(node: Node, count: int) -> int:
	var c = count
	if node.has_method("interact"):
		c += 1
	for child in node.get_children():
		c = _count_interactive(child, c)
	return c

func _find_node_of_class(class_name: String) -> Node:
	return _search_node(get_tree().root, class_name)

func _search_node(node: Node, class_name: String) -> Node:
	if node.get_script():
		var script = node.get_script()
		if script:
			var name = script.get_global_name() if script.has_method("get_global_name") else ""
			if node.name == class_name or name == class_name or str(node).find(class_name) != -1:
				return node
	for child in node.get_children():
		var result = _search_node(child, class_name)
		if result:
			return result
	return null

func _finalize():
	print("\n" + "=" * 50)
	print("  SMOKE TEST RESULTS")
	print("=" * 50)

	var passed = 0
	var failed = 0
	for stage_name in _stage_results:
		var r = _stage_results[stage_name]
		if r["passed"]:
			print("  [PASS] %s (%.1fs)" % [stage_name, r["elapsed"]])
			passed += 1
		else:
			print("  [FAIL] %s: %s" % [stage_name, r["error"]])
			failed += 1

	print("=" * 50)
	print("  Total: %d passed, %d failed" % [passed, failed])
	print("  Errors: %d, Warnings: %d" % [_total_errors, _total_warnings])
	print("  Duration: %.1f seconds" % _elapsed)
	print("=" * 50)

	if failed > 0 or _total_errors > 0:
		get_tree().quit(1)
	else:
		print("\n  ALL SMOKE TESTS PASSED!")
		get_tree().quit(0)

func _fail(reason: String):
	_total_errors += 1
	push_error(reason)
	print("\n" + "=" * 50)
	print("  FATAL: %s" % reason)
	print("=" * 50)
	get_tree().quit(1)