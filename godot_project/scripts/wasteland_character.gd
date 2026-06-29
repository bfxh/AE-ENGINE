extends CharacterBody3D
class_name WastelandCharacter

enum CharacterState { IDLE, WALK, RUN, JUMP, FALL, ATTACK, HIT, DEAD }
enum CharacterFaction { PLAYER, WANDERER, RAIDER, MUTANT, GHOST, ROBOT, ANIMAL }

@export var character_name: String = "Unknown"
@export var faction: CharacterFaction = CharacterFaction.WANDERER
@export var max_health: float = 100.0
@export var move_speed: float = 5.0
@export var run_speed: float = 10.0
@export var jump_velocity: float = 8.0
@export var rotation_speed: float = 10.0
@export var attack_range: float = 2.0
@export var attack_damage: float = 15.0
@export var detection_range: float = 30.0

var health: float
var current_state: CharacterState = CharacterState.IDLE
var target: Node3D = null
var nav_agent: NavigationAgent3D
var animation_tree: AnimationTree
var animation_state: AnimationNodeStateMachinePlayback
var skeleton: Skeleton3D
var gravity: float = ProjectSettings.get_setting("physics/3d/default_gravity") if ProjectSettings.has_setting("physics/3d/default_gravity") else 9.8

var _movement_target: Vector3
var _has_movement_target: bool = false
var _attack_cooldown: float = 0.0
var _state_time: float = 0.0
var _wander_timer: float = 0.0
var _wander_interval: float = 5.0
var _spawn_position: Vector3

signal health_changed(new_health: float)
signal character_died()
signal state_changed(old_state: CharacterState, new_state: CharacterState)
signal target_acquired(target: Node3D)

func _ready():
	health = max_health
	_spawn_position = global_position

	nav_agent = NavigationAgent3D.new()
	nav_agent.path_desired_distance = 1.0
	nav_agent.target_desired_distance = 1.0
	add_child(nav_agent)

	_initialize_animation_system()

func _initialize_animation_system():
	var model = $Model
	if not model:
		model = _find_model_node()
		if not model:
			return

	skeleton = model.get_node_or_null("Skeleton3D")
	if not skeleton:
		for child in model.get_children():
			if child is Skeleton3D:
				skeleton = child
				break

	animation_tree = AnimationTree.new()
	animation_tree.name = "AnimationTree"
	add_child(animation_tree)

	var anim_player = model.get_node_or_null("AnimationPlayer")
	if anim_player:
		animation_tree.anim_player = anim_player.path

		var root_motion = AnimationNodeBlendTree.new()
		animation_tree.tree_root = root_motion

		var state_machine = AnimationNodeStateMachine.new()
		animation_tree.tree_root = AnimationNodeBlendTree.new()

	_swap_state(CharacterState.IDLE)

func _find_model_node() -> Node3D:
	for child in get_children():
		if child is MeshInstance3D:
			return child
		if child is Node3D and child.name.to_lower().find("model") != -1:
			return child
	return null

func _physics_process(delta: float):
	_state_time += delta

	if current_state == CharacterState.DEAD:
		return

	match current_state:
		CharacterState.IDLE:
			_process_idle(delta)
		CharacterState.WALK:
			_process_movement(delta, move_speed)
		CharacterState.RUN:
			_process_movement(delta, run_speed)
		CharacterState.JUMP:
			_process_jump(delta)
		CharacterState.FALL:
			_process_fall(delta)
		CharacterState.ATTACK:
			_process_attack(delta)
		CharacterState.HIT:
			_process_hit(delta)

	_apply_gravity(delta)
	move_and_slide()

func _process_idle(delta: float):
	_wander_timer += delta

	if target and _can_see_target():
		if global_position.distance_to(target.global_position) <= attack_range:
			_swap_state(CharacterState.ATTACK)
		else:
			_set_movement_target(target.global_position)
			_swap_state(CharacterState.RUN)
		return

	if _has_movement_target:
		_swap_state(CharacterState.WALK)
		return

	if _wander_timer > _wander_interval:
		_wander_to_random_point()
		_wander_timer = 0.0
		_wander_interval = randf_range(3.0, 10.0)

func _process_movement(delta: float, speed: float):
	if not _has_movement_target:
		_swap_state(CharacterState.IDLE)
		return

	var current_pos = global_position
	var next_pos = nav_agent.get_next_path_position()

	var direction = (next_pos - current_pos).normalized()
	if direction.length() > 0.01:
		velocity.x = direction.x * speed
		velocity.z = direction.z * speed

		var look_dir = Vector3(direction.x, 0, direction.z)
		if look_dir.length() > 0.01:
			var target_rotation = atan2(look_dir.x, look_dir.z)
			rotation.y = lerp_angle(rotation.y, target_rotation, rotation_speed * delta)

	if current_pos.distance_to(_movement_target) < 2.0:
		_has_movement_target = false
		_swap_state(CharacterState.IDLE)

func _process_jump(delta: float):
	velocity.y = jump_velocity
	_swap_state(CharacterState.FALL)

func _process_fall(delta: float):
	if is_on_floor():
		if _has_movement_target:
			_swap_state(CharacterState.WALK)
		else:
			_swap_state(CharacterState.IDLE)

func _process_attack(delta: float):
	_attack_cooldown -= delta
	if _attack_cooldown <= 0:
		if target and _can_see_target() and global_position.distance_to(target.global_position) <= attack_range:
			_deal_damage_to_target()
			_attack_cooldown = 1.0
		else:
			_swap_state(CharacterState.IDLE)

func _process_hit(delta: float):
	if _state_time > 0.5:
		if target:
			_swap_state(CharacterState.RUN)
		else:
			_swap_state(CharacterState.IDLE)

func _apply_gravity(delta: float):
	if not is_on_floor():
		velocity.y -= gravity * delta

func _swap_state(new_state: CharacterState):
	if current_state == new_state:
		return

	var old_state = current_state
	current_state = new_state
	_state_time = 0.0
	state_changed.emit(old_state, new_state)

func _set_movement_target(target_pos: Vector3):
	nav_agent.target_position = target_pos
	_movement_target = target_pos
	_has_movement_target = true

func _wander_to_random_point():
	var wander_radius = 20.0
	var angle = randf() * TAU
	var distance = randf_range(5.0, wander_radius)
	var target_pos = _spawn_position + Vector3(cos(angle) * distance, 0, sin(angle) * distance)
	_set_movement_target(target_pos)

func _can_see_target() -> bool:
	if not target:
		return false

	var distance = global_position.distance_to(target.global_position)
	if distance > detection_range:
		return false

	var space_state = get_world_3d().direct_space_state
	var query = PhysicsRayQueryParameters3D.create(
		global_position + Vector3.UP,
		target.global_position + Vector3.UP
	)
	query.exclude = [self]
	var result = space_state.intersect_ray(query)

	if result and result.collider == target:
		return true

	return false

func _deal_damage_to_target():
	if target.has_method("take_damage"):
		target.take_damage(attack_damage, self)

func take_damage(amount: float, attacker: Node = null):
	if current_state == CharacterState.DEAD:
		return

	health -= amount
	health_changed.emit(health)

	if not target and attacker:
		target = attacker

	if health <= 0:
		health = 0
		die()
	elif _state_time < 0.2:
		_swap_state(CharacterState.HIT)

func die():
	_swap_state(CharacterState.DEAD)
	character_died.emit()

func get_character_data() -> Dictionary:
	return {
		"name": character_name,
		"faction": faction,
		"health": health,
		"max_health": max_health,
		"state": current_state,
		"position": global_position,
		"has_target": target != null,
	}

func restore_character_data(data: Dictionary):
	character_name = data.get("name", character_name)
	faction = data.get("faction", faction)
	health = data.get("health", max_health)
	max_health = data.get("max_health", max_health)

	if data.has("position") and data["position"] is Vector3:
		global_position = data["position"]

	var state_str = data.get("state", "IDLE")
	if state_str in CharacterState.values():
		current_state = CharacterState[state_str]