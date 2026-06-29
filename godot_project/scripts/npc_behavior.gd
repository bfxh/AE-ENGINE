extends Node3D
class_name NPCBehavior

enum BehaviorState { IDLE, PATROL, FOLLOW, FLEE, ATTACK, INVESTIGATE, WORK, REST, EAT, TRADE }
enum NPCRole { SURVIVOR, MERCHANT, GUARD, SCAVENGER, HOSTILE, ANIMAL }

@export var npc_role: NPCRole = NPCRole.SURVIVOR
@export var detection_range: float = 20.0
@export var attack_range: float = 2.0
@export var patrol_range: float = 30.0
@export var move_speed: float = 3.0

var current_state: BehaviorState = BehaviorState.IDLE
var target: Node3D = null
var home_position: Vector3 = Vector3.ZERO
var patrol_points: Array = []
var current_patrol_index: int = 0
var state_timer: float = 0.0
var alert_level: float = 0.0
var health: float = 100.0
var max_health: float = 100.0
var hostility: float = 0.0
var fear: float = 0.0
var hunger_level: float = 0.0
var fatigue: float = 0.0

var _movement: MovementFSM
var _body: CharacterBody3D
var _nav_region: NavigationRegion3D

signal state_changed(from: BehaviorState, to: BehaviorState)
signal npc_damaged(amount: float)
signal npc_died()

const ROLE_CONFIG = {
	NPCRole.SURVIVOR: {"hostility_base": 0.1, "fear_base": 0.3, "trade": true, "patrol": true},
	NPCRole.MERCHANT: {"hostility_base": 0.0, "fear_base": 0.5, "trade": true, "patrol": false},
	NPCRole.GUARD: {"hostility_base": 0.2, "fear_base": 0.1, "trade": false, "patrol": true},
	NPCRole.SCAVENGER: {"hostility_base": 0.3, "fear_base": 0.4, "trade": true, "patrol": true},
	NPCRole.HOSTILE: {"hostility_base": 0.9, "fear_base": 0.1, "trade": false, "patrol": true},
	NPCRole.ANIMAL: {"hostility_base": 0.5, "fear_base": 0.6, "trade": false, "patrol": true},
}

func _ready():
	home_position = global_position
	_movement = MovementFSM.new()
	add_child(_movement)

	var config = ROLE_CONFIG.get(npc_role, ROLE_CONFIG[NPCRole.SURVIVOR])
	hostility = config["hostility_base"]
	fear = config["fear_base"]

	_generate_patrol_points()

func _generate_patrol_points():
	patrol_points.clear()
	var num_points = randi_range(3, 6)
	for i in range(num_points):
		var angle = (TAU / num_points) * i + randf() * 0.5
		var dist = randf_range(5, patrol_range)
		patrol_points.append(home_position + Vector3(cos(angle) * dist, 0, sin(angle) * dist))

func _physics_process(delta):
	state_timer += delta
	_update_needs(delta)
	_detect_player()

	match current_state:
		BehaviorState.IDLE:
			_handle_idle(delta)
		BehaviorState.PATROL:
			_handle_patrol(delta)
		BehaviorState.FOLLOW:
			_handle_follow(delta)
		BehaviorState.FLEE:
			_handle_flee(delta)
		BehaviorState.ATTACK:
			_handle_attack(delta)
		BehaviorState.INVESTIGATE:
			_handle_investigate(delta)
		BehaviorState.WORK:
			_handle_work(delta)
		BehaviorState.REST:
			_handle_rest(delta)
		BehaviorState.EAT:
			_handle_eat(delta)
		BehaviorState.TRADE:
			_handle_trade(delta)

func _update_needs(delta):
	hunger_level = min(100, hunger_level + 0.5 * delta)
	fatigue = min(100, fatigue + 0.3 * delta)

	if hunger_level > 70 and current_state == BehaviorState.IDLE:
		_set_state(BehaviorState.EAT)
	elif fatigue > 80 and current_state == BehaviorState.IDLE:
		_set_state(BehaviorState.REST)

func _detect_player():
	var players = get_tree().get_nodes_in_group("player")
	if players.is_empty():
		target = null
		return

	var player = players[0] as Node3D
	if not player:
		return

	var dist = global_position.distance_to(player.global_position)

	if dist < detection_range:
		if not target:
			target = player
			alert_level = min(1.0, alert_level + 0.3)

		if hostility > 0.6 and dist < attack_range:
			_set_state(BehaviorState.ATTACK)
		elif hostility > 0.6 and dist < detection_range * 0.7:
			_set_state(BehaviorState.FOLLOW)
		elif fear > 0.5 and dist < detection_range * 0.5:
			_set_state(BehaviorState.FLEE)
		elif alert_level > 0.5:
			_set_state(BehaviorState.INVESTIGATE)
	else:
		if target:
			target = null
			alert_level = max(0, alert_level - 0.1)

func _handle_idle(delta):
	if state_timer > randf_range(3, 8):
		var config = ROLE_CONFIG.get(npc_role, ROLE_CONFIG[NPCRole.SURVIVOR])
		if config["patrol"]:
			_set_state(BehaviorState.PATROL)
		else:
			state_timer = 0

func _handle_patrol(delta):
	if patrol_points.is_empty():
		_set_state(BehaviorState.IDLE)
		return

	var target_pos = patrol_points[current_patrol_index]
	var dist = global_position.distance_to(target_pos)

	if dist < 1.0:
		current_patrol_index = (current_patrol_index + 1) % patrol_points.size()
		if randf() > 0.5:
			_set_state(BehaviorState.IDLE)
			return
	else:
		_move_toward(target_pos, delta)

func _handle_follow(delta):
	if not target or not is_instance_valid(target):
		_set_state(BehaviorState.IDLE)
		return

	var dist = global_position.distance_to(target.global_position)
	if dist > detection_range * 1.5:
		_set_state(BehaviorState.IDLE)
		return

	if dist > attack_range * 0.8:
		_move_toward(target.global_position, delta)

func _handle_flee(delta):
	if not target or not is_instance_valid(target):
		_set_state(BehaviorState.IDLE)
		return

	var away_dir = (global_position - target.global_position).normalized()
	var flee_target = global_position + away_dir * 10.0
	_move_toward(flee_target, delta)

	if state_timer > 5.0:
		_set_state(BehaviorState.IDLE)

func _handle_attack(delta):
	if not target or not is_instance_valid(target):
		_set_state(BehaviorState.IDLE)
		return

	var dist = global_position.distance_to(target.global_position)
	if dist > attack_range:
		_move_toward(target.global_position, delta)
	elif state_timer > 1.0:
		if target.has_method("take_damage"):
			var damage = randf_range(5, 15)
			target.take_damage(damage)
		state_timer = 0

	if dist > detection_range:
		_set_state(BehaviorState.IDLE)

func _handle_investigate(delta):
	if target and is_instance_valid(target):
		var dist = global_position.distance_to(target.global_position)
		if dist > 3.0:
			_move_toward(target.global_position, delta)
		else:
			alert_level = max(0, alert_level - 0.2 * delta)
			if alert_level < 0.2:
				_set_state(BehaviorState.IDLE)

	if state_timer > 10.0:
		_set_state(BehaviorState.IDLE)

func _handle_work(delta):
	if state_timer > randf_range(10, 30):
		_set_state(BehaviorState.IDLE)

func _handle_rest(delta):
	fatigue = max(0, fatigue - 2.0 * delta)
	if fatigue < 20 or state_timer > 20:
		_set_state(BehaviorState.IDLE)

func _handle_eat(delta):
	hunger_level = max(0, hunger_level - 3.0 * delta)
	if hunger_level < 20 or state_timer > 10:
		_set_state(BehaviorState.IDLE)

func _handle_trade(delta):
	if state_timer > 30:
		_set_state(BehaviorState.IDLE)

func _move_toward(target_pos: Vector3, delta: float):
	var direction = (target_pos - global_position)
	direction.y = 0
	direction = direction.normalized()

	var input = Vector2(direction.x, direction.z)
	_movement.process_movement(delta, input, false, false, false)

	position += direction * move_speed * delta

	if direction != Vector3.ZERO:
		var target_rot = atan2(direction.x, direction.z)
		rotation.y = lerp_angle(rotation.y, target_rot, 5.0 * delta)

func _set_state(new_state: BehaviorState):
	if current_state == new_state:
		return
	var from = current_state
	current_state = new_state
	state_timer = 0.0
	state_changed.emit(from, new_state)

func take_damage(amount: float):
	health -= amount
	npc_damaged.emit(amount)

	if health <= 0:
		health = 0
		npc_died.emit()
		queue_free()
	elif fear > 0.3:
		_set_state(BehaviorState.FLEE)
	else:
		hostility = min(1.0, hostility + 0.2)
		_set_state(BehaviorState.ATTACK)

func interact(player: Node3D) -> Dictionary:
	var config = ROLE_CONFIG.get(npc_role, ROLE_CONFIG[NPCRole.SURVIVOR])

	if config["trade"]:
		_set_state(BehaviorState.TRADE)
		return {"type": "trade", "available": true}

	if hostility > 0.5:
		return {"type": "hostile", "message": "Get away!"}

	return {"type": "talk", "message": "Hello, stranger."}

func get_debug_info() -> Dictionary:
	return {
		"role": NPCRole.keys()[npc_role],
		"state": BehaviorState.keys()[current_state],
		"health": health,
		"hostility": hostility,
		"fear": fear,
		"alert": alert_level,
		"hunger": hunger_level,
		"fatigue": fatigue,
		"has_target": target != null,
	}
