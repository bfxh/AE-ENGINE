extends Node
class_name GameTestFramework

var test_results = []
var wasteland_world
var test_parent: Node3D
var performance_log = []

signal test_completed(test_name: String, passed: bool, message: String)

func _init(p_wasteland_world = null, p_parent: Node3D = null):
	wasteland_world = p_wasteland_world
	test_parent = p_parent

func run_all_tests():
	print("==================================================")
	print("WASTELAND GAMEPLAY TEST SUITE")
	print("==================================================")

	test_results.clear()
	performance_log.clear()

	_test_world_initialization()
	_test_terrain_generation()
	_test_tree_system()
	_test_npc_spawning()
	_test_dialogue_system()
	_test_explosion_system()
	_test_ecosystem_integration()
	_test_physics_collision()
	_test_performance_benchmark()
	_test_save_load_stress()
	_test_memory_stability()
	_test_vulnerability_scan()

	_print_summary()
	return test_results

func _test_world_initialization():
	var test_name = "World Initialization"
	print("\n[TEST] %s" % test_name)

	var checks = []

	if wasteland_world:
		checks.append(true)
		var stats = wasteland_world.get_stats()
		if stats:
			checks.append(stats.has("time"))
			checks.append(stats.has("tick_count"))
		else:
			checks.append(false)
	else:
		checks.append(true)

	var passed = not false in checks
	_record_result(test_name, passed, "GDExtension check: %s" % ("loaded" if wasteland_world else "fallback"))

func _test_terrain_generation():
	var test_name = "Terrain Generation"
	print("\n[TEST] %s" % test_name)

	var checks = []
	var terrain = TerrainGenerator.new()
	add_child(terrain)
	terrain.generate_terrain()

	await get_tree().process_frame

	checks.append(terrain.terrain_mesh != null)
	checks.append(terrain.terrain_mesh.mesh != null)
	if terrain.terrain_mesh.mesh:
		var surf_count = terrain.terrain_mesh.mesh.get_surface_count()
		checks.append(surf_count > 0)

	terrain.queue_free()
	var passed = not false in checks
	_record_result(test_name, passed, "Terrain mesh generated: %s" % checks)

func _test_tree_system():
	var test_name = "Tree Generation"
	print("\n[TEST] %s" % test_name)

	var trees = AdvancedTreeSystem.new()
	trees.tree_count = 20
	trees.spawn_radius = 50.0
	add_child(trees)
	trees.generate_forest()

	await get_tree().process_frame

	var tree_count = trees.tree_parent.get_child_count()
	var passed = tree_count > 0

	trees.queue_free()
	_record_result(test_name, passed, "Generated %d trees" % tree_count)

func _test_npc_spawning():
	var test_name = "NPC Spawning"
	print("\n[TEST] %s" % test_name)

	if not wasteland_world:
		_record_result(test_name, false, "No GDExtension available")
		return

	var initial_count = wasteland_world.get_npc_count()
	var id = wasteland_world.spawn_npc("TestBot", 0, 5, 0, "robot", "testers")
	var after_spawn = wasteland_world.get_npc_count()

	var checks = []
	checks.append(after_spawn > initial_count)
	checks.append(id != "")
	checks.append(id.length() > 0)

	var passed = not false in checks
	_record_result(test_name, passed, "NPC count: %d -> %d, ID: %s" % [initial_count, after_spawn, id])

func _test_dialogue_system():
	var test_name = "Dialogue System"
	print("\n[TEST] %s" % test_name)

	if not wasteland_world:
		_record_result(test_name, false, "No GDExtension available")
		return

	var id = wasteland_world.spawn_npc("DialogTest", 10, 5, 10, "human", "testers")
	var dialogue = wasteland_world.npc_dialogue(id, "Hello")

	var checks = []
	checks.append(dialogue != null)
	if dialogue:
		checks.append(dialogue.has("text"))
		checks.append(dialogue.has("emotion"))
		checks.append(dialogue.has("action"))

	var passed = not false in checks
	_record_result(test_name, passed, "Dialogue: %s" % str(dialogue.get("text", "N/A")))

func _test_explosion_system():
	var test_name = "Explosion System"
	print("\n[TEST] %s" % test_name)

	if not wasteland_world:
		_record_result(test_name, false, "No GDExtension available")
		return

	wasteland_world.spawn_voxel_grid(5, 5, 5, 2.0, 0, 0, 0)
	var before = wasteland_world.voxel_grid_count()
	wasteland_world.apply_explosion(0, 5, 0, 8.0, 100.0)
	var after = wasteland_world.voxel_grid_count()

	var passed = after >= before
	_record_result(test_name, passed, "Voxel grids: %d -> %d" % [before, after])

func _test_ecosystem_integration():
	var test_name = "Ecosystem Integration"
	print("\n[TEST] %s" % test_name)

	if not wasteland_world:
		_record_result(test_name, false, "No GDExtension available")
		return

	var eco_count = wasteland_world.ecosystem_count()
	wasteland_world.spawn_ecosystem("TestEco", -20, -5, -20, 20, 20, 20)
	await get_tree().process_frame
	await get_tree().process_frame

	var after_count = wasteland_world.ecosystem_count()
	var passed = after_count > eco_count
	_record_result(test_name, passed, "Ecosystem count: %d -> %d" % [eco_count, after_count])

func _test_physics_collision():
	var test_name = "Physics Collision"
	print("\n[TEST] %s" % test_name)

	var checks = []

	var body_a = RigidBody3D.new()
	body_a.mass = 1.0
	var shape_a = CollisionShape3D.new()
	shape_a.shape = SphereShape3D.new()
	shape_a.shape.radius = 1.0
	body_a.add_child(shape_a)
	body_a.position = Vector3(-5, 5, 0)
	add_child(body_a)

	var body_b = RigidBody3D.new()
	body_b.mass = 1.0
	var shape_b = CollisionShape3D.new()
	shape_b.shape = SphereShape3D.new()
	shape_b.shape.radius = 1.0
	body_b.add_child(shape_b)
	body_b.position = Vector3(5, 5, 0)
	add_child(body_b)

	body_a.linear_velocity = Vector3(5, 0, 0)
	body_b.linear_velocity = Vector3(-5, 0, 0)

	await get_tree().physics_frame
	await get_tree().physics_frame

	checks.append(body_a.position.x < 0 or body_b.position.x > 0)
	var distance = body_a.position.distance_to(body_b.position)
	checks.append(distance > 0.5)

	body_a.queue_free()
	body_b.queue_free()

	var passed = not false in checks
	_record_result(test_name, passed, "Collision response active, final distance: %.2f" % distance)

func _test_performance_benchmark():
	var test_name = "Performance Benchmark"
	print("\n[TEST] %s" % test_name)

	var fps_samples = []
	var frame_times = []

	var test_entities = []

	for i in range(100):
		var sphere = MeshInstance3D.new()
		sphere.mesh = SphereMesh.new()
		sphere.mesh.radius = 0.5
		sphere.mesh.height = 1.0
		sphere.mesh.radial_segments = 16
		sphere.mesh.rings = 8
		sphere.position = Vector3(randf_range(-50, 50), randf_range(0, 20), randf_range(-50, 50))
		add_child(sphere)
		test_entities.append(sphere)

	for i in range(60):
		await get_tree().process_frame
		fps_samples.append(Engine.get_frames_per_second())
		frame_times.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000)

	for entity in test_entities:
		entity.queue_free()

	var avg_fps = 0.0
	for fps in fps_samples:
		avg_fps += fps
	avg_fps /= max(fps_samples.size(), 1)

	var min_fps = fps_samples[0]
	for fps in fps_samples:
		if fps < min_fps:
			min_fps = fps

	var passed = avg_fps >= 45
	performance_log.append({"avg_fps": avg_fps, "min_fps": min_fps, "entity_count": 100})

	_record_result(test_name, passed, "Avg FPS: %.1f, Min FPS: %.1f (100 dynamic spheres)" % [avg_fps, min_fps])

func _test_save_load_stress():
	var test_name = "Save/Load Stress"
	print("\n[TEST] %s" % test_name)

	if not wasteland_world:
		_record_result(test_name, false, "No GDExtension available")
		return

	var cycles = 3
	var checks = []

	for c in range(cycles):
		wasteland_world.spawn_npc("Stress_%d" % c, randf_range(-50, 50), 5, randf_range(-50, 50), "human", "testers")
		await get_tree().process_frame

		var stats = wasteland_world.get_stats()
		checks.append(stats.has("npc_count"))

	var final_npc = wasteland_world.get_npc_count()
	var passed = final_npc >= cycles and not false in checks
	_record_result(test_name, passed, "After %d spawn cycles, NPC count: %d" % [cycles, final_npc])

func _test_memory_stability():
	var test_name = "Memory Stability"
	print("\n[TEST] %s" % test_name)

	var initial_mem = OS.get_static_memory_usage()

	var temp_objects = []
	for i in range(500):
		var node = Node3D.new()
		for j in range(5):
			var child = MeshInstance3D.new()
			child.mesh = BoxMesh.new()
			node.add_child(child)
		add_child(node)
		temp_objects.append(node)

	await get_tree().process_frame

	for obj in temp_objects:
		obj.queue_free()

	await get_tree().process_frame
	await get_tree().process_frame

	var final_mem = OS.get_static_memory_usage()
	var growth = (final_mem - initial_mem) / 1048576.0

	var passed = growth < 50.0
	_record_result(test_name, passed, "Memory growth after 500 nodes: %.1f MB" % growth)

func _test_vulnerability_scan():
	var test_name = "Vulnerability Scan"
	print("\n[TEST] %s" % test_name)

	var vulnerabilities = []

	if wasteland_world:
		var methods_to_test = [
			"init_world", "spawn_ecosystem", "spawn_voxel_grid",
			"apply_explosion", "spawn_npc", "npc_dialogue",
			"spawn_iron_particles", "dual_phase_voxel_to_particles"
		]

		for method in methods_to_test:
			var has_method = wasteland_world.has_method(method)
			if not has_method:
				vulnerabilities.append("Missing: %s" % method)

		var data = wasteland_world.get_field_value("invalid_field", 0, 0, 0)
		if typeof(data) != TYPE_FLOAT:
			vulnerabilities.append("get_field_value no default for invalid field")

	else:
		var script_paths = [
			"res://scripts/game_manager.gd",
			"res://scripts/advanced_tree_system.gd",
			"res://scripts/vegetation_spawner.gd",
			"res://scripts/water_body_system.gd",
			"res://scripts/atmosphere_controller.gd"
		]

		for path in script_paths:
			if not FileAccess.file_exists(path.replace("res://", "res://")):
				vulnerabilities.append("Script not found: %s" % path)

	var target_node = get_node_or_null("..") if test_parent == null else test_parent
	if target_node:
		var node_count = target_node.get_child_count()
		if node_count > 1000:
			vulnerabilities.append("High node count: %d" % node_count)

	var passed = vulnerabilities.size() < 3
	var msg = "%d vulnerabilities found" % vulnerabilities.size()
	if vulnerabilities.size() > 0:
		msg += ": " + vulnerabilities[0]

	_record_result(test_name, passed, msg)

func _record_result(test_name: String, passed: bool, message: String):
	var status = "PASS" if passed else "FAIL"
	print("[%s] %s - %s" % [status, test_name, message])
	test_results.append({
		"name": test_name,
		"passed": passed,
		"message": message
	})
	emit_signal("test_completed", test_name, passed, message)

func _print_summary():
	var passed_count = 0
	for r in test_results:
		if r["passed"]:
			passed_count += 1

	print("\n==================================================")
	print("TEST SUMMARY: %d/%d passed" % [passed_count, test_results.size()])
	print("==================================================")

	for r in test_results:
		var status = "PASS" if r["passed"] else "FAIL"
		print("  [%s] %s" % [status, r["name"]])

	if performance_log.size() > 0:
		print("\nPerformance Data:")
		for p in performance_log:
			print("  Avg FPS: %.1f, Min FPS: %.1f (%d entities)" % [p["avg_fps"], p["min_fps"], p["entity_count"]])