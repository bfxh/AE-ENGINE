extends Node
class_name MovementFSM

enum State { IDLE, WALK, RUN, DRIVE, FLY, SWIM, FALL, CLIMB, CROUCH, SLIDE }

var current_state: State = State.IDLE
var previous_state: State = State.IDLE
var state_time: float = 0.0
var velocity: Vector3 = Vector3.ZERO
var is_grounded: bool = true

var config: Dictionary = {
	"walk_speed": 4.0,
	"run_speed": 8.0,
	"drive_speed": 20.0,
	"fly_speed": 15.0,
	"swim_speed": 3.0,
	"climb_speed": 2.5,
	"crouch_speed": 2.0,
	"slide_speed": 12.0,
	"gravity": 20.0,
	"jump_force": 8.0,
	"acceleration": 15.0,
	"deceleration": 20.0,
	"turn_speed": 8.0,
	"max_slope_angle": 45.0,
	"stamina_drain_run": 10.0,
	"stamina_drain_climb": 15.0,
	"stamina_drain_swim": 8.0,
	"stamina_regen": 5.0,
}

var stamina: float = 100.0
var max_stamina: float = 100.0

signal state_changed(from: State, to: State)
signal movement_processed(velocity: Vector3, state: State)
signal stamina_changed(current: float, maximum: float)

var _transitions: Dictionary = {}
var _state_handlers: Dictionary = {}

func _ready():
	_setup_transitions()
	_setup_handlers()

func _setup_transitions():
	_transitions = {
		[State.IDLE, "move"] = State.WALK,
		[State.IDLE, "jump"] = State.FALL,
		[State.IDLE, "crouch"] = State.CROUCH,
		[State.IDLE, "enter_vehicle"] = State.DRIVE,
		[State.IDLE, "start_fly"] = State.FLY,
		[State.IDLE, "enter_water"] = State.SWIM,
		[State.IDLE, "start_climb"] = State.CLIMB,
		[State.WALK, "stop"] = State.IDLE,
		[State.WALK, "sprint"] = State.RUN,
		[State.WALK, "jump"] = State.FALL,
		[State.WALK, "crouch"] = State.CROUCH,
		[State.WALK, "enter_vehicle"] = State.DRIVE,
		[State.WALK, "enter_water"] = State.SWIM,
		[State.WALK, "start_climb"] = State.CLIMB,
		[State.RUN, "stop"] = State.IDLE,
		[State.RUN, "walk"] = State.WALK,
		[State.RUN, "jump"] = State.FALL,
		[State.RUN, "exhausted"] = State.WALK,
		[State.RUN, "enter_water"] = State.SWIM,
		[State.FALL, "land"] = State.IDLE,
		[State.FALL, "enter_water"] = State.SWIM,
		[State.FALL, "start_climb"] = State.CLIMB,
		[State.DRIVE, "exit_vehicle"] = State.IDLE,
		[State.DRIVE, "enter_water"] = State.SWIM,
		[State.FLY, "stop_fly"] = State.FALL,
		[State.FLY, "land"] = State.IDLE,
		[State.SWIM, "exit_water"] = State.IDLE,
		[State.SWIM, "dive"] = State.SWIM,
		[State.CLIMB, "stop_climb"] = State.FALL,
		[State.CLIMB, "reach_top"] = State.IDLE,
		[State.CLIMB, "exhausted"] = State.FALL,
		[State.CROUCH, "stand"] = State.IDLE,
		[State.CROUCH, "move"] = State.CROUCH,
		[State.CROUCH, "sprint"] = State.SLIDE,
		[State.SLIDE, "stop"] = State.CROUCH,
		[State.SLIDE, "stand"] = State.IDLE,
	}

func _setup_handlers():
	_state_handlers = {
		State.IDLE: _handle_idle,
		State.WALK: _handle_walk,
		State.RUN: _handle_run,
		State.DRIVE: _handle_drive,
		State.FLY: _handle_fly,
		State.SWIM: _handle_swim,
		State.FALL: _handle_fall,
		State.CLIMB: _handle_climb,
		State.CROUCH: _handle_crouch,
		State.SLIDE: _handle_slide,
	}

func transition(event: String) -> bool:
	var key = [current_state, event]
	if _transitions.has(key):
		var from = current_state
		var to = _transitions[key]
		_exit_state(current_state)
		previous_state = from
		current_state = to
		state_time = 0.0
		_enter_state(to)
		state_changed.emit(from, to)
		return true
	return false

func try_transition(events: Array) -> bool:
	for event in events:
		if transition(event):
			return true
	return false

func _enter_state(state: State):
	pass

func _exit_state(state: State):
	pass

func process_movement(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool) -> Vector3:
	state_time += delta

	_regen_stamina(delta)

	if _state_handlers.has(current_state):
		_state_handlers[current_state].call(delta, input_dir, want_jump, want_sprint, want_crouch)

	if not is_grounded and current_state != State.FLY and current_state != State.SWIM and current_state != State.CLIMB:
		velocity.y -= config["gravity"] * delta

	movement_processed.emit(velocity, current_state)
	return velocity

func _handle_idle(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	velocity.x = move_toward(velocity.x, 0, config["deceleration"] * delta)
	velocity.z = move_toward(velocity.z, 0, config["deceleration"] * delta)

	if input_dir != Vector2.ZERO:
		transition("move")
	elif want_jump and is_grounded:
		velocity.y = config["jump_force"]
		is_grounded = false
		transition("jump")
	elif want_crouch:
		transition("crouch")

func _handle_walk(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	if input_dir == Vector2.ZERO:
		transition("stop")
		return

	var speed = config["walk_speed"]
	_apply_horizontal_movement(delta, input_dir, speed)

	if want_sprint and stamina > 0:
		transition("sprint")
	elif want_jump and is_grounded:
		velocity.y = config["jump_force"]
		is_grounded = false
		transition("jump")
	elif want_crouch:
		transition("crouch")

func _handle_run(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	if input_dir == Vector2.ZERO:
		transition("stop")
		return

	if not want_sprint or stamina <= 0:
		transition("walk")
		return

	var speed = config["run_speed"]
	_apply_horizontal_movement(delta, input_dir, speed)
	_drain_stamina(config["stamina_drain_run"] * delta)

	if want_jump and is_grounded:
		velocity.y = config["jump_force"] * 1.1
		is_grounded = false
		transition("jump")

func _handle_drive(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["drive_speed"]
	_apply_horizontal_movement(delta, input_dir, speed)
	velocity.y = move_toward(velocity.y, 0, config["deceleration"] * delta)

func _handle_fly(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["fly_speed"]
	velocity.y = 0

	if want_jump:
		velocity.y = speed * 0.5
	elif want_crouch:
		velocity.y = -speed * 0.5

	_apply_horizontal_movement(delta, input_dir, speed)

func _handle_swim(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["swim_speed"]
	_drain_stamina(config["stamina_drain_swim"] * delta)

	velocity.y = 0
	if want_jump:
		velocity.y = speed * 0.6
	elif want_crouch:
		velocity.y = -speed * 0.6

	_apply_horizontal_movement(delta, input_dir, speed)

func _handle_fall(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["walk_speed"] * 0.5
	_apply_horizontal_movement(delta, input_dir, speed)

func _handle_climb(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["climb_speed"]
	_drain_stamina(config["stamina_drain_climb"] * delta)

	velocity = Vector3.ZERO
	velocity.y = input_dir.y * speed
	velocity.x = input_dir.x * speed * 0.5

	if stamina <= 0:
		transition("exhausted")

func _handle_crouch(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["crouch_speed"]
	_apply_horizontal_movement(delta, input_dir, speed)

	if not want_crouch:
		transition("stand")
	elif want_sprint and input_dir != Vector2.ZERO:
		transition("sprint")

func _handle_slide(delta: float, input_dir: Vector2, want_jump: bool, want_sprint: bool, want_crouch: bool):
	var speed = config["slide_speed"]
	state_time += 0

	var decay = 1.0 - (state_time * 2.0)
	if decay <= 0.1:
		transition("stop")
		return

	velocity.x *= decay
	velocity.z *= decay

	if not want_crouch:
		transition("stand")

func _apply_horizontal_movement(delta: float, input_dir: Vector2, speed: float):
	var target = Vector3(input_dir.x, 0, input_dir.y).normalized() * speed
	velocity.x = move_toward(velocity.x, target.x, config["acceleration"] * delta)
	velocity.z = move_toward(velocity.z, target.z, config["acceleration"] * delta)

func _drain_stamina(amount: float):
	stamina = max(0, stamina - amount)
	stamina_changed.emit(stamina, max_stamina)

	if stamina <= 0 and current_state == State.RUN:
		transition("exhausted")

func _regen_stamina(delta: float):
	if current_state == State.IDLE or current_state == State.WALK:
		stamina = min(max_stamina, stamina + config["stamina_regen"] * delta)
		stamina_changed.emit(stamina, max_stamina)

func set_grounded(value: bool):
	if value and not is_grounded:
		is_grounded = true
		if current_state == State.FALL:
			transition("land")
	elif not value and is_grounded:
		is_grounded = false
		if current_state != State.FLY and current_state != State.SWIM and current_state != State.CLIMB:
			transition("jump")

func set_in_water(value: bool):
	if value:
		transition("enter_water")
	elif current_state == State.SWIM:
		transition("exit_water")

func get_state_name() -> String:
	return State.keys()[current_state]

func get_speed() -> float:
	match current_state:
		State.IDLE:
			return 0.0
		State.WALK:
			return config["walk_speed"]
		State.RUN:
			return config["run_speed"]
		State.DRIVE:
			return config["drive_speed"]
		State.FLY:
			return config["fly_speed"]
		State.SWIM:
			return config["swim_speed"]
		State.FALL:
			return config["walk_speed"] * 0.5
		State.CLIMB:
			return config["climb_speed"]
		State.CROUCH:
			return config["crouch_speed"]
		State.SLIDE:
			return config["slide_speed"]
		_:
			return 0.0

func get_debug_info() -> Dictionary:
	return {
		"state": State.keys()[current_state],
		"previous_state": State.keys()[previous_state],
		"state_time": state_time,
		"velocity": velocity,
		"grounded": is_grounded,
		"stamina": stamina,
		"speed": velocity.length(),
	}
