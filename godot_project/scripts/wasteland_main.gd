extends Node3D

var world_manager: WorldManager
var environment_system: EnvironmentSystem
var inventory_system: InventorySystem
var game_hud: GameHUD
var player: PlayerController
var ground: MeshInstance3D

func _ready():
	print("=== WASTELAND v1.0 ===")
	print("Controls: WASD=Move | Space=Jump | Shift=Sprint | Ctrl=Crouch")
	print("E=Interact | F=Fly | V=Vehicle | T=Debug | I=Inventory | Tab=Weather")

	_init_systems()
	_create_ground()
	_generate_world()
	_spawn_player()
	_setup_hud()

	print("Scene ready! Systems: SchemaLoader + MetaEntityBuilder + EntitySpawner + MovementFSM + Environment + Inventory + HUD + NPC AI")

func _init_systems():
	world_manager = WorldManager.new()
	world_manager.name = "WorldManager"
	add_child(world_manager)

	environment_system = EnvironmentSystem.new()
	environment_system.name = "EnvironmentSystem"
	add_child(environment_system)

	inventory_system = InventorySystem.new()
	inventory_system.name = "InventorySystem"
	add_child(inventory_system)

func _create_ground():
	ground = MeshInstance3D.new()
	ground.name = "Ground"

	var mesh = PlaneMesh.new()
	mesh.size = Vector2(200, 200)
	ground.mesh = mesh

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(0.3, 0.4, 0.2)
	ground.material_override = mat

	ground.position = Vector3(0, 0, 0)
	ground.rotation = Vector3(PI/2, 0, 0)
	add_child(ground)

	var ground_body = StaticBody3D.new()
	ground_body.name = "GroundCollision"
	var ground_shape = CollisionShape3D.new()
	ground_shape.shape = PlaneShape3D.new()
	ground_body.add_child(ground_shape)
	ground.add_child(ground_body)

func _generate_world():
	var result = world_manager.generate_wasteland_scene(90.0)
	print("World generated: ", result)

func _spawn_player():
	player = PlayerController.new()
	player.name = "Player"
	player.add_to_group("player")

	var body = MeshInstance3D.new()
	var body_mesh = BoxMesh.new()
	body_mesh.size = Vector3(0.4, 1.6, 0.25)
	body.mesh = body_mesh

	var body_mat = StandardMaterial3D.new()
	body_mat.albedo_color = Color(0.3, 0.6, 0.8)
	body.material_override = body_mat
	body.position = Vector3(0, 0.8, 0)
	player.add_child(body)

	var head = MeshInstance3D.new()
	var head_mesh = SphereMesh.new()
	head_mesh.radius = 0.22
	head.mesh = head_mesh

	var head_mat = StandardMaterial3D.new()
	head_mat.albedo_color = Color(0.92, 0.85, 0.75)
	head.material_override = head_mat
	head.position = Vector3(0, 1.75, 0)
	player.add_child(head)

	player.position = Vector3(0, 1, 0)
	add_child(player)

	print("Player spawned with MovementFSM + Camera + Interaction")

func _setup_hud():
	game_hud = GameHUD.new()
	game_hud.name = "GameHUD"
	add_child(game_hud)
	game_hud.setup(player, inventory_system, environment_system)

func _input(event):
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_I:
				_toggle_inventory()
			KEY_TAB:
				environment_system.set_random_weather()
			KEY_R:
				_run_comprehensive_tests()
			KEY_ESCAPE:
				if Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
					Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				else:
					Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)

func _toggle_inventory():
	var items = inventory_system.get_all_items()
	if items.is_empty():
		inventory_system.add_item("scrap_metal", 5)
		inventory_system.add_item("water_bottle", 2)
		inventory_system.add_item("ammo_9mm", 30)
		inventory_system.add_item("medkit", 1)
		inventory_system.add_item("knife", 1)
		game_hud.show_message("Starter kit added!")
	else:
		var info = inventory_system.get_debug_info()
		game_hud.show_message("Inventory: %d items, %.1f/%.1f kg" % [info["items"], info["weight"], info["weight_max"]])

func _run_comprehensive_tests():
	print("=== COMPREHENSIVE TESTS v1.0 ===")
	var passed = 0
	var total = 10

	var t1 = world_manager != null
	print("[1] WorldManager: " + str(t1))
	passed += 1 if t1 else 0

	var t2 = world_manager.schema_loader != null
	print("[2] SchemaLoader: " + str(t2))
	passed += 1 if t2 else 0

	var t3 = world_manager.entity_spawner != null
	print("[3] EntitySpawner: " + str(t3))
	passed += 1 if t3 else 0

	var t4 = player != null and player.movement != null
	print("[4] Player + MovementFSM: " + str(t4))
	passed += 1 if t4 else 0

	var t5 = environment_system != null
	print("[5] EnvironmentSystem: " + str(t5))
	passed += 1 if t5 else 0

	var t6 = inventory_system != null
	print("[6] InventorySystem: " + str(t6))
	passed += 1 if t6 else 0

	var t7 = game_hud != null
	print("[7] GameHUD: " + str(t7))
	passed += 1 if t7 else 0

	var t8 = world_manager.entity_spawner.get_all_spawned_ids().size() > 0
	print("[8] Entities spawned: " + str(t8))
	passed += 1 if t8 else 0

	var t9 = inventory_system.add_item("scrap_metal", 1)
	print("[9] Inventory add: " + str(t9))
	passed += 1 if t9 else 0

	var t10 = environment_system.get_environment_data().size() > 0
	print("[10] Environment data: " + str(t10))
	passed += 1 if t10 else 0

	print("=== TESTS: %d/%d ===" % [passed, total])

	if passed == total:
		game_hud.show_message("All %d tests passed!" % total)
	else:
		game_hud.show_message("Tests: %d/%d passed" % [passed, total])
