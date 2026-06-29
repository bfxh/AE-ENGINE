extends Node3D

const TICK_RATE: float = 1.0 / 60.0
const SPAWN_COUNT: int = 200
const WORLD_SIZE: float = 50.0

var accumulator: float = 0.0
var tick_count: int = 0
var fps_label: Label

func _ready():
	print("[Wasteland] Unified Engine Demo v0.3")
	print("[Wasteland] Architecture: MetaEntity + UnifiedInterface + Scheduler + Registry")

	setup_camera()
	setup_ui()
	spawn_entities()
	print("[Wasteland] Spawned %d entities" % SPAWN_COUNT)

func setup_camera():
	var cam = $Camera3D
	if cam:
		cam.position = Vector3(0, 20, 30)
		cam.look_at(Vector3.ZERO, Vector3.UP)

func setup_ui():
	var canvas = CanvasLayer.new()
	canvas.name = "UI"
	add_child(canvas)

	fps_label = Label.new()
	fps_label.position = Vector2(10, 10)
	fps_label.add_theme_font_size_override("font_size", 16)
	fps_label.add_theme_color_override("font_color", Color.GREEN)
	canvas.add_child(fps_label)

	var info_label = Label.new()
	info_label.name = "InfoLabel"
	info_label.position = Vector2(10, 35)
	info_label.add_theme_font_size_override("font_size", 14)
	info_label.add_theme_color_override("font_color", Color.WHITE)
	canvas.add_child(info_label)

func spawn_entities():
	var rng = RandomNumberGenerator.new()
	rng.randomize()

	for i in range(SPAWN_COUNT):
		var x = rng.randf_range(-WORLD_SIZE, WORLD_SIZE)
		var y = rng.randf_range(-5, 5)
		var z = rng.randf_range(-WORLD_SIZE, WORLD_SIZE)
		spawn_cube(x, y, z, rng)

	var ground = CSGBox3D.new()
	ground.size = Vector3(WORLD_SIZE * 2.5, 0.5, WORLD_SIZE * 2.5)
	ground.position = Vector3(0, -3, 0)
	ground.material = StandardMaterial3D.new()
	ground.material.albedo_color = Color(0.3, 0.25, 0.2)
	add_child(ground)

func spawn_cube(x: float, y: float, z: float, rng: RandomNumberGenerator):
	var cube = CSGBox3D.new()
	var size = rng.randf_range(0.5, 2.0)
	cube.size = Vector3(size, size, size)
	cube.position = Vector3(x, y, z)
	cube.material = StandardMaterial3D.new()
	cube.material.albedo_color = Color(
		rng.randf_range(0.2, 0.8),
		rng.randf_range(0.2, 0.8),
		rng.randf_range(0.2, 0.8),
		1.0
	)
	cube.set_meta("entity_type", rng.randi_range(0, 5))
	cube.set_meta("mass", size * size * size * 2700.0)
	cube.set_meta("velocity", Vector3.ZERO)
	cube.set_meta("temperature", rng.randf_range(273.0, 373.0))
	cube.set_meta("ph", rng.randf_range(0.0, 14.0))
	cube.set_meta("health", 100.0)
	cube.set_meta("version", 0)
	add_child(cube)

func _process(delta):
	accumulator += delta

	while accumulator >= TICK_RATE:
		tick(TICK_RATE)
		accumulator -= TICK_RATE
		tick_count += 1

	update_ui(delta)
	handle_input(delta)

func tick(dt: float):
	var entities = get_children().filter(func(c): return c is CSGBox3D)

	for entity in entities:
		var vel: Vector3 = entity.get_meta("velocity", Vector3.ZERO)
		entity.position += vel * dt

		var temp = entity.get_meta("temperature", 300.0)
		temp = max(temp - 0.01 * dt, 0.0)
		entity.set_meta("temperature", temp)

		var version = entity.get_meta("version", 0)
		entity.set_meta("version", version + 1)

		if entity.position.y < -10:
			entity.position.y = randi_range(5, 15)
			entity.set_meta("velocity", Vector3.ZERO)

	if tick_count % 120 == 0:
		print("[Wasteland] Tick %d | Entities: %d" % [tick_count, entities.size()])

func update_ui(delta):
	if fps_label:
		fps_label.text = "FPS: %d | Tick: %d | Entities: %d" % [
			Engine.get_frames_per_second(), tick_count, get_child_count()
		]

	var info = get_node_or_null("UI/InfoLabel")
	if info:
		var total_versions = 0
		var avg_temp = 0.0
		var entities = get_children().filter(func(c): return c is CSGBox3D)
		for e in entities:
			total_versions += e.get_meta("version", 0)
			avg_temp += e.get_meta("temperature", 300.0)
		if entities.size() > 0:
			avg_temp /= entities.size()
		info.text = "Avg Temp: %.1fK | Total Versions: %d" % [avg_temp, total_versions]

func handle_input(delta):
	if Input.is_action_just_pressed("ui_accept"):
		var rng = RandomNumberGenerator.new()
		rng.randomize()
		for i in range(20):
			spawn_cube(
				rng.randf_range(-10, 10),
				rng.randf_range(5, 15),
				rng.randf_range(-10, 10),
				rng
			)
		print("[Wasteland] Spawned 20 entities")

	if Input.is_action_just_pressed("ui_cancel"):
		get_tree().quit()

	if Input.is_key_pressed(KEY_R):
		var entities = get_children().filter(func(c): return c is CSGBox3D)
		for e in entities:
			e.queue_free()
		spawn_entities()
		print("[Wasteland] Reset complete")