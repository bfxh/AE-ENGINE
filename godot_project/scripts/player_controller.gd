extends CharacterBody3D
class_name PlayerController

@export var camera_sensitivity: float = 0.003
@export var camera_distance: float = 5.0
@export var camera_height: float = 2.5
@export var camera_smooth: float = 8.0

var movement: MovementFSM
var camera_pivot: Node3D
var camera: Camera3D
var interaction_ray: RayCast3D

var _input_dir: Vector2 = Vector2.ZERO
var _want_jump: bool = false
var _want_sprint: bool = false
var _want_crouch: bool = false
var _camera_yaw: float = 0.0
var _camera_pitch: float = -0.3

signal interact_pressed(target: Node3D)
signal debug_info_requested()

func _ready():
	movement = MovementFSM.new()
	add_child(movement)

	_setup_camera()
	_setup_interaction()
	_setup_input()

func _setup_camera():
	camera_pivot = Node3D.new()
	camera_pivot.name = "CameraPivot"
	add_child(camera_pivot)

	camera = Camera3D.new()
	camera.name = "Camera"
	camera.fov = 75.0
	camera.current = true
	camera.position = Vector3(0, camera_height, -camera_distance)
	camera_pivot.add_child(camera)

	camera_pivot.rotation.x = _camera_pitch

func _setup_interaction():
	interaction_ray = RayCast3D.new()
	interaction_ray.name = "InteractionRay"
	interaction_ray.target_position = Vector3(0, 0, -3.0)
	interaction_ray.collision_mask = 1
	add_child(interaction_ray)

func _setup_input():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)

func _physics_process(delta):
	_gather_input()
	movement.set_grounded(is_on_floor())

	var vel = movement.process_movement(delta, _input_dir, _want_jump, _want_sprint, _want_crouch)

	velocity = vel
	if movement.current_state == MovementFSM.State.FLY:
		velocity.y = vel.y
	elif not is_on_floor():
		velocity.y = vel.y

	move_and_slide()

	_update_camera(delta)

	if _want_jump:
		_want_jump = false

func _gather_input():
	_input_dir = Vector2.ZERO
	if Input.is_key_pressed(KEY_D) or Input.is_key_pressed(KEY_RIGHT):
		_input_dir.x += 1
	if Input.is_key_pressed(KEY_A) or Input.is_key_pressed(KEY_LEFT):
		_input_dir.x -= 1
	if Input.is_key_pressed(KEY_W) or Input.is_key_pressed(KEY_UP):
		_input_dir.y -= 1
	if Input.is_key_pressed(KEY_S) or Input.is_key_pressed(KEY_DOWN):
		_input_dir.y += 1

	_input_dir = _input_dir.normalized()

	_want_sprint = Input.is_key_pressed(KEY_SHIFT)
	_want_crouch = Input.is_key_pressed(KEY_CTRL)

func _input(event):
	if event is InputEventMouseMotion and Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		_camera_yaw -= event.relative.x * camera_sensitivity
		_camera_pitch -= event.relative.y * camera_sensitivity
		_camera_pitch = clamp(_camera_pitch, -1.2, 0.8)

	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_SPACE:
				_want_jump = true
			KEY_E:
				_try_interact()
			KEY_F:
				movement.transition("start_fly") if movement.current_state != MovementFSM.State.FLY else movement.transition("stop_fly")
			KEY_ESCAPE:
				if Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
					Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				else:
					Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
			KEY_T:
				debug_info_requested.emit()
			KEY_V:
				movement.transition("enter_vehicle") if movement.current_state != MovementFSM.State.DRIVE else movement.transition("exit_vehicle")

func _update_camera(delta):
	camera_pivot.rotation.y = lerp_angle(camera_pivot.rotation.y, _camera_yaw, camera_smooth * delta)
	camera_pivot.rotation.x = lerp(camera_pivot.rotation.x, _camera_pitch, camera_smooth * delta)

	camera.position = Vector3(0, camera_height, -camera_distance)

	var forward = -camera_pivot.global_transform.basis.z
	forward.y = 0
	forward = forward.normalized()

	if _input_dir != Vector2.ZERO:
		var move_dir = (forward * -_input_dir.y + camera_pivot.global_transform.basis.x * _input_dir.x).normalized()
		var target_rot_y = atan2(move_dir.x, move_dir.z)
		rotation.y = lerp_angle(rotation.y, target_rot_y, movement.config["turn_speed"] * delta)

func _try_interact():
	if interaction_ray.is_colliding():
		var collider = interaction_ray.get_collider()
		if collider:
			var parent = _find_interactable_parent(collider)
			if parent:
				interact_pressed.emit(parent)

func _find_interactable_parent(node: Node) -> Node3D:
	var current = node
	while current:
		if current.has_node("InteractionData"):
			return current as Node3D
		current = current.get_parent()
	return null

func get_debug_info() -> Dictionary:
	var info = movement.get_debug_info()
	info["position"] = global_position
	info["camera_yaw"] = _camera_yaw
	info["camera_pitch"] = _camera_pitch
	return info

func take_damage(amount: float):
	if movement._meta_node and movement._meta_node.has_method("apply_damage_to_entity"):
		pass

func heal(amount: float):
	pass

func has_tool(tool_name: String) -> bool:
	return false

func try_unlock(difficulty: int) -> bool:
	return randf() < 0.3 * difficulty
