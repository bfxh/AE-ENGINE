extends CharacterBody3D
class_name AnimalController

enum AnimalType {
	DEER, WOLF, BEAR, BOAR, RABBIT,
	BIRD, FOX, COYOTE,
}

enum AnimalState { IDLE, GRAZING, WALKING, FLEEING, HUNTING, SLEEPING, DEAD }

var animal_type: AnimalType
var current_state: AnimalState = AnimalState.IDLE
var health: float = 100.0
var max_health: float = 100.0
var move_speed: float = 3.0
var flee_speed: float = 8.0
var detection_range: float = 25.0
var flee_range: float = 40.0
var home_position: Vector3
var wander_radius: float = 30.0

var nav_agent: NavigationAgent3D
var _state_timer: float = 0.0
var _animation_tree: AnimationTree
var _model: Node3D
var _threats: Array = []
var _prey: Node3D = null

var species_config = {
	AnimalType.DEER: {
		"max_health": 80.0, "move_speed": 5.0, "flee_speed": 12.0,
		"detection_range": 30.0, "flee_range": 50.0, "wander_radius": 40.0,
		"diet": "herbivore", "herd_animal": true, "can_attack": false,
		"model_scale": 1.0, "day_active": true,
	},
	AnimalType.WOLF: {
		"max_health": 120.0, "move_speed": 6.0, "flee_speed": 15.0,
		"detection_range": 40.0, "flee_range": 30.0, "wander_radius": 60.0,
		"diet": "carnivore", "herd_animal": true, "can_attack": true,
		"model_scale": 0.9, "day_active": false,
	},
	AnimalType.BEAR: {
		"max_health": 300.0, "move_speed": 4.0, "flee_speed": 10.0,
		"detection_range": 25.0, "flee_range": 35.0, "wander_radius": 50.0,
		"diet": "omnivore", "herd_animal": false, "can_attack": true,
		"model_scale": 1.8, "day_active": true,
	},
	AnimalType.BOAR: {
		"max_health": 150.0, "move_speed": 5.0, "flee_speed": 11.0,
		"detection_range": 20.0, "flee_range": 35.0, "wander_radius": 25.0,
		"diet": "omnivore", "herd_animal": false, "can_attack": true,
		"model_scale": 0.7, "day_active": true,
	},
	AnimalType.RABBIT: {
		"max_health": 30.0, "move_speed": 3.0, "flee_speed": 14.0,
		"detection_range": 15.0, "flee_range": 40.0, "wander_radius": 15.0,
		"diet": "herbivore", "herd_animal": false, "can_attack": false,
		"model_scale": 0.3, "day_active": true,
	},
	AnimalType.BIRD: {
		"max_health": 20.0, "move_speed": 4.0, "flee_speed": 20.0,
		"detection_range": 20.0, "flee_range": 60.0, "wander_radius": 80.0,
		"diet": "omnivore", "herd_animal": false, "can_attack": false,
		"model_scale": 0.15, "day_active": true,
	},
	AnimalType.FOX: {
		"max_health": 60.0, "move_speed": 6.0, "flee_speed": 14.0,
		"detection_range": 30.0, "flee_range": 45.0, "wander_radius": 35.0,
		"diet": "carnivore", "herd_animal": false, "can_attack": true,
		"model_scale": 0.5, "day_active": true,
	},
	AnimalType.COYOTE: {
		"max_health": 90.0, "move_speed": 7.0, "flee_speed": 15.0,
		"detection_range": 35.0, "flee_range": 30.0, "wander_radius": 70.0,
		"diet": "carnivore", "herd_animal": true, "can_attack": true,
		"model_scale": 0.65, "day_active": false,
	},
}

func _init(p_type: AnimalType, p_position: Vector3):
	animal_type = p_type
	home_position = p_position
	var cfg = species_config[p_type]
	max_health = cfg["max_health"]
	health = max_health
	move_speed = cfg["move_speed"]
	flee_speed = cfg["flee_speed"]
	detection_range = cfg["detection_range"]
	flee_range = cfg["flee_range"]
	wander_radius = cfg["wander_radius"]

func _ready():
	nav_agent = NavigationAgent3D.new()
	nav_agent.path_desired_distance = 1.5
	nav_agent.target_desired_distance = 2.0
	add_child(nav_agent)

	_setup_model()
	_swap_state(AnimalState.IDLE)
	_wander()

func _setup_model():
	var mesh = MeshInstance3D.new()
	mesh.name = "AnimalModel"

	match animal_type:
		AnimalType.DEER:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(1.5, 1.2, 0.8)
			mesh.material_override = _create_material(Color(0.55, 0.35, 0.15))
		AnimalType.WOLF:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(1.2, 0.8, 0.6)
			mesh.material_override = _create_material(Color(0.25, 0.25, 0.3))
		AnimalType.BEAR:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(2.0, 1.4, 1.2)
			mesh.material_override = _create_material(Color(0.2, 0.15, 0.1))
		AnimalType.BOAR:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(1.0, 0.7, 0.6)
			mesh.material_override = _create_material(Color(0.15, 0.1, 0.05))
		AnimalType.RABBIT:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(0.4, 0.3, 0.3)
			mesh.material_override = _create_material(Color(0.6, 0.5, 0.4))
		AnimalType.BIRD:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(0.3, 0.2, 0.2)
			mesh.material_override = _create_material(Color(0.2, 0.3, 0.5))
		AnimalType.FOX:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(0.8, 0.5, 0.5)
			mesh.material_override = _create_material(Color(0.8, 0.4, 0.1))
		AnimalType.COYOTE:
			mesh.mesh = BoxMesh.new()
			mesh.scale = Vector3(1.0, 0.6, 0.5)
			mesh.material_override = _create_material(Color(0.4, 0.35, 0.3))

	add_child(mesh)
	_model = mesh

func _create_material(color: Color) -> StandardMaterial3D:
	var mat = StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = 0.8
	return mat

func _process(delta: float):
	if current_state == AnimalState.DEAD:
		return

	_state_timer += delta
	_scan_for_threats()

	match current_state:
		AnimalState.IDLE:
			_process_idle(delta)
		AnimalState.GRAZING:
			_process_grazing(delta)
		AnimalState.WALKING:
			_process_walking(delta)
		AnimalState.FLEEING:
			_process_fleeing(delta)
		AnimalState.HUNTING:
			_process_hunting(delta)
		AnimalState.SLEEPING:
			_process_sleeping(delta)

	move_and_slide()

func _scan_for_threats():
	var space_state = get_world_3d().direct_space_state
	var nearby = PhysicsShapeQueryParameters3D.new()
	nearby.shape = SphereShape3D.new()
	nearby.shape.radius = detection_range
	nearby.transform.origin = global_position

	var results = space_state.intersect_shape(nearby)
	_threats.clear()

	var cfg = species_config[animal_type]
	for result in results:
		var obj = result.get("collider")
		if not obj:
			continue
		if obj.has_method("take_damage"):
			var char_name = obj.get("character_name") if obj.get("character_name") else ""
			if "player" in char_name.to_lower() or "raider" in char_name.to_lower():
				_threats.append(obj)

	if cfg["diet"] == "carnivore" and _prey == null:
		for result in results:
			var obj = result.get("collider")
			if obj is AnimalController and obj != self:
				var prey_cfg = species_config[obj.animal_type]
				if prey_cfg["diet"] == "herbivore" and obj.health > 0:
					_prey = obj
					break

func _process_idle(delta: float):
	if not _threats.is_empty():
		_swap_state(AnimalState.FLEEING)
		return

	if _prey and _can_hunt_prey():
		_swap_state(AnimalState.HUNTING)
		return

	if _state_timer > randf_range(2.0, 5.0):
		var roll = randf()
		if roll < 0.4:
			_swap_state(AnimalState.GRAZING)
		elif roll < 0.7:
			_swap_state(AnimalState.WALKING)
		_state_timer = 0.0

func _process_grazing(delta: float):
	if not _threats.is_empty():
		_swap_state(AnimalState.FLEEING)
		return

	if _state_timer > randf_range(3.0, 8.0):
		_swap_state(AnimalState.IDLE)
		_state_timer = 0.0

func _process_walking(delta: float):
	if not _threats.is_empty():
		_swap_state(AnimalState.FLEEING)
		return

	if nav_agent.is_navigation_finished() or _state_timer > 10.0:
		_swap_state(AnimalState.IDLE)
		_state_timer = 0.0

func _process_fleeing(delta: float):
	if _threats.is_empty():
		_swap_state(AnimalState.IDLE)
		return

	var nearest_threat = _threats[0]
	var nearest_dist = global_position.distance_to(nearest_threat.global_position)
	for threat in _threats:
		var dist = global_position.distance_to(threat.global_position)
		if dist < nearest_dist:
			nearest_dist = dist
			nearest_threat = threat

	var flee_dir = (global_position - nearest_threat.global_position).normalized()
	var flee_target = global_position + flee_dir * flee_range

	if nearest_dist > flee_range:
		_swap_state(AnimalState.IDLE)
	else:
		nav_agent.target_position = flee_target
		var direction = (nav_agent.get_next_path_position() - global_position).normalized()
		if direction.length() > 0.01:
			velocity = direction * flee_speed

func _process_hunting(delta: float):
	if _prey == null or _prey.health <= 0:
		_prey = null
		_swap_state(AnimalState.IDLE)
		return

	nav_agent.target_position = _prey.global_position
	var direction = (nav_agent.get_next_path_position() - global_position).normalized()
	if direction.length() > 0.01:
		velocity = direction * move_speed

	if global_position.distance_to(_prey.global_position) < 2.0:
		_attack_prey()

func _process_sleeping(delta: float):
	if not _threats.is_empty():
		_swap_state(AnimalState.FLEEING)
		return

	if _state_timer > randf_range(10.0, 30.0):
		_swap_state(AnimalState.IDLE)
		_state_timer = 0.0

func _swap_state(new_state: AnimalState):
	if current_state == new_state:
		return
	current_state = new_state
	_state_timer = 0.0

func _wander():
	var angle = randf() * TAU
	var distance = randf_range(5.0, wander_radius)
	var target = home_position + Vector3(cos(angle) * distance, 0, sin(angle) * distance)
	nav_agent.target_position = target

func _can_hunt_prey() -> bool:
	if not _prey:
		return false
	var cfg = species_config[animal_type]
	return cfg["can_attack"] and cfg["diet"] == "carnivore"

func _attack_prey():
	if _prey and _prey.has_method("take_damage"):
		_prey.take_damage(25.0)
		if _prey.health <= 0:
			_prey = null

func take_damage(amount: float):
	health -= amount
	if health <= 0:
		health = 0
		die()

func die():
	_swap_state(AnimalState.DEAD)
	queue_free()

func get_animal_data() -> Dictionary:
	return {
		"type": animal_type,
		"state": current_state,
		"health": health,
		"max_health": max_health,
		"position": global_position,
		"home": home_position,
		"diet": species_config[animal_type]["diet"],
	}