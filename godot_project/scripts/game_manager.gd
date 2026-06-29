extends Node3D
class_name GameManager

var camera: Camera3D
var wasteland_world
var stats_label: Label3D
var debug_label: Label3D
var time_scale: float = 1.0
var paused: bool = false
var show_debug: bool = true
var frame_count: int = 0
var world_size: float = 500.0
var target_fps: int = 60

func _ready():
	camera = $Camera3D
	if camera:
		camera.current = true

	_setup_extensions()
	_setup_ui()
	spawn_initial_ecosystems()
	_setup_lights()

	print("Wasteland Forest Environment v0.3")
	print("World: ", world_size, "m x ", world_size, "m")
	print("WASD: Move | Mouse: Look | Scroll: Zoom")
	print("P: Pause | F: Toggle Stats | L: Toggle Lights")
	print("1: Spawn NPC | 2: Explosion | 3: Spawn Animals")

func _setup_extensions():
	var ext_path = "res://bin/wasteland_gdextension.dll"
	if FileAccess.file_exists(ext_path):
		var world_node = Node3D.new()
		world_node.name = "WastelandWorld"
		world_node.set_script(load("res://wasteland.gdextension"))
		add_child(world_node)
		wasteland_world = world_node
		wasteland_world.init_world(world_size * 0.5)
		print("[GDExtension] Engine DLL loaded")
	else:
		print("[Fallback] No GDExtension DLL, running pure Godot mode")

func _setup_ui():
	var ui_parent = Node3D.new()
	ui_parent.name = "UI"
	add_child(ui_parent)

	stats_label = Label3D.new()
	stats_label.name = "StatsLabel"
	stats_label.position = Vector3(0, 25, -40)
	stats_label.font_size = 36
	stats_label.outline_size = 4
	stats_label.modulate = Color(0.9, 0.9, 0.9, 0.9)
	stats_label.billboard = BaseMaterial3D.BILLBOARD_FIXED_Y
	ui_parent.add_child(stats_label)

	debug_label = Label3D.new()
	debug_label.name = "DebugLabel"
	debug_label.position = Vector3(0, 20, -40)
	debug_label.font_size = 24
	debug_label.outline_size = 3
	debug_label.modulate = Color(0.6, 0.9, 0.6, 0.8)
	debug_label.billboard = BaseMaterial3D.BILLBOARD_FIXED_Y
	ui_parent.add_child(debug_label)

func _setup_lights():
	var sun = $DirectionalLight3D
	if sun:
		sun.light_energy = 1.5
		sun.light_color = Color(1.0, 0.85, 0.6)
		sun.shadow_enabled = true
		sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
		sun.directional_shadow_split_1 = 0.1
		sun.directional_shadow_split_2 = 0.3
		sun.directional_shadow_split_3 = 0.6

	var env = WorldEnvironment.new()
	env.name = "WorldEnv"
	var env_res = Environment.new()
	env_res.background_mode = Environment.BG_SKY
	var sky = Sky.new()
	sky.sky_material = ProceduralSkyMaterial.new()
	sky.sky_material.sun_angle_max = 80.0
	sky.sky_material.ground_horizon_color = Color(0.3, 0.4, 0.2)
	sky.sky_material.ground_bottom_color = Color(0.1, 0.15, 0.05)
	env_res.sky = sky
	env_res.sky_rotation = Vector3(0.0, 0.2, 0.0)
	env_res.volumetric_fog_enabled = true
	env_res.volumetric_fog_density = 0.008
	env_res.volumetric_fog_albedo = Color(0.85, 0.7, 0.5)
	env_res.volumetric_fog_emission = Color(0.2, 0.2, 0.1)
	env_res.ssao_enabled = true
	env_res.ssil_enabled = true
	env_res.glow_enabled = true
	env_res.glow_bloom = 0.1
	env_res.adjustment_enabled = true
	env_res.adjustment_contrast = 1.1
	env_res.adjustment_saturation = 1.05
	env.resource = env_res
	add_child(env)

func spawn_initial_ecosystems():
	if wasteland_world:
		wasteland_world.spawn_ecosystem("Central Forest", -100, -5, -100, 100, 30, 100)
		wasteland_world.spawn_ecosystem("Western Woods", -200, -5, -100, -100, 25, 100)
		wasteland_world.spawn_ecosystem("Eastern Grove", 100, -5, -100, 200, 28, 100)
		wasteland_world.spawn_ecosystem("Northern Pines", -100, -5, -200, 100, 20, -100)
		wasteland_world.spawn_ecosystem("Southern Wetlands", -100, -10, 100, 100, 15, 200)
		print("[Ecosystem] 5 biomes spawned")

func _process(delta):
	frame_count += 1
	_handle_input(delta)

	if paused:
		return

	_update_stats()
	if frame_count % 30 == 0:
		_run_diagnostics()

func _handle_input(delta):
	if Input.is_action_just_pressed("toggle_pause"):
		paused = not paused
		print("Paused: ", paused)
	if Input.is_action_just_pressed("toggle_debug"):
		show_debug = not show_debug
		_show_debug_label()

	if paused:
		return

	var dt = delta * time_scale
	if Input.is_action_just_pressed("speed_up"):
		time_scale = min(time_scale + 0.5, 10.0)
	if Input.is_action_just_pressed("speed_down"):
		time_scale = max(time_scale - 0.5, 0.1)

	var move_dir = Vector3.ZERO
	if Input.is_action_pressed("move_forward"): move_dir.z -= 1
	if Input.is_action_pressed("move_back"): move_dir.z += 1
	if Input.is_action_pressed("move_left"): move_dir.x -= 1
	if Input.is_action_pressed("move_right"): move_dir.x += 1

	if move_dir.length() > 0 and camera:
		move_dir = move_dir.normalized()
		var speed = 40.0
		var forward = -camera.global_transform.basis.z.normalized()
		forward.y = 0
		forward = forward.normalized()
		var right = camera.global_transform.basis.x.normalized()
		var movement = (forward * move_dir.z + right * move_dir.x) * speed * dt
		camera.global_translate(movement)

	if Input.is_action_pressed("jump") and camera:
		camera.global_translate(Vector3.UP * 20.0 * dt)

	if camera and Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT):
		var mouse_delta = Input.get_last_mouse_velocity()
		camera.rotate_y(-mouse_delta.x * 0.002)
		camera.rotate_x(-mouse_delta.y * 0.002)
		camera.rotation.x = clamp(camera.rotation.x, -PI / 2, PI / 2)

	if Input.is_action_just_pressed("action_npc_spawn"):
		_spawn_test_npc()
	if Input.is_action_just_pressed("action_explosion"):
		_trigger_test_explosion()
	if Input.is_action_just_pressed("action_spawn_animals"):
		_spawn_test_animals()

func _input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP and camera:
			camera.global_translate(-camera.global_transform.basis.z * 8.0)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and camera:
			camera.global_translate(camera.global_transform.basis.z * 8.0)

func _update_stats():
	if not stats_label:
		return

	var fps = Engine.get_frames_per_second()
	var entities = 0
	var ecosystems = 0
	var npc_count = 0
	var mem_mb = OS.get_static_memory_usage() / 1048576.0
	var video_mb = Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1048576.0
	var draw_calls = Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	var objects = Performance.get_monitor(Performance.OBJECT_NODE_COUNT)
	var physics_time = Performance.get_monitor(Performance.PHYSICS_3D_ACTIVE_OBJECTS)

	if wasteland_world:
		var stats = wasteland_world.get_stats()
		if stats:
			entities = stats.get("meta_entity_count", 0)
			ecosystems = stats.get("ecosystem_count", 0)
			npc_count = stats.get("npc_count", 0)

	stats_label.text = "FPS: %d | Time: %.1fx | Objects: %d | NPCs: %d | Ecos: %d | Entities: %d" % [fps, time_scale, objects, npc_count, ecosystems, entities]

	if show_debug:
		var fps_str = "%3d fps" % fps
		var color = Color.GREEN
		if fps < 30:
			color = Color.RED
		elif fps < 50:
			color = Color.YELLOW

		debug_label.modulate = color
		debug_label.position = camera.global_position + camera.global_transform.basis * Vector3(0, 5, -30)
		debug_label.text = "FPS: %s | Mem: %.1fMB | VRAM: %.1fMB\nDraws: %d | Phys: %d | Tick: %d" % [fps_str, mem_mb, video_mb, draw_calls, physics_time, frame_count]

func _show_debug_label():
	debug_label.visible = show_debug

func _run_diagnostics():
	if not wasteland_world:
		return

	var voxel_count = wasteland_world.voxel_grid_count()
	var particle_count = wasteland_world.get_particle_count()
	var event_count = wasteland_world.get_event_count()
	var cache_hit = wasteland_world.get_cache_stats().get("hit_rate", 0.0)
	var health = wasteland_world.get_health_report()
	var critical = health.get("critical", 0)

	if critical > 0:
		push_warning("[DIAG] %d critical health monitors" % critical)
	if cache_hit < 0.5:
		push_warning("[DIAG] Low cache hit rate: %.1f%%" % (cache_hit * 100))

func _spawn_test_npc():
	if not wasteland_world:
		return

	var species_list = ["human", "mutant", "ghoul", "robot"]
	var species = species_list[randi() % species_list.size()]
	var faction = "wanderers" if randi() % 2 == 0 else "raiders"

	var rng = RandomNumberGenerator.new()
	rng.randomize()
	var r = rng.randf_range(30, 200)
	var angle = rng.randf() * TAU
	var px = cos(angle) * r
	var pz = sin(angle) * r

	var id = wasteland_world.spawn_npc("Test_NPC_%d" % frame_count, px, 5, pz, species, faction)
	var dialogue = wasteland_world.npc_dialogue(id, "Hello, stranger.")
	print("[NPC] Spawned %s (id=%s) at (%.0f, %.0f)" % [species, id, px, pz])
	if dialogue and dialogue.has("text"):
		print("[NPC] Says: ", dialogue["text"])

func _trigger_test_explosion():
	if not wasteland_world:
		return

	var rng = RandomNumberGenerator.new()
	rng.randomize()
	var x = rng.randf_range(-200, 200)
	var y = rng.randf_range(2, 10)
	var z = rng.randf_range(-200, 200)
	var radius = rng.randf_range(5, 20)
	var force = rng.randf_range(50, 200)

	wasteland_world.apply_explosion(x, y, z, radius, force)
	print("[EXPLOSION] at (%.0f, %.0f, %.0f) r=%.0f force=%.0f" % [x, y, z, radius, force])

func _spawn_test_animals():
	if not wasteland_world:
		return

	for i in range(10):
		var id = wasteland_world.spawn_npc(
			"Animal_%d_%d" % [frame_count, i],
			randf_range(-200, 200), 1, randf_range(-200, 200),
			"animal", "wildlife"
		)
	print("[Animals] Spawned 10 wildlife NPCs")