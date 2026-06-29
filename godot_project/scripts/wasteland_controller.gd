extends Node3D

var camera: Camera3D
var time_scale: float = 1.0
var paused: bool = false

func _ready():
	camera = $Camera3D
	if camera:
		camera.current = true
	print("Wasteland Engine v0.2 - Running")
	print("WASD: Move | Mouse: Look | P: Pause | +/-: Speed")

func _process(delta):
	if Input.is_action_just_pressed("toggle_pause"):
		paused = not paused
		print("Paused: ", paused)
	if paused:
		return

	if Input.is_action_just_pressed("speed_up"):
		time_scale = min(time_scale + 0.5, 10.0)
		print("Time Scale: ", time_scale)
	if Input.is_action_just_pressed("speed_down"):
		time_scale = max(time_scale - 0.5, 0.1)
		print("Time Scale: ", time_scale)

	var dt = delta * time_scale

	var move_dir = Vector3.ZERO
	if Input.is_action_pressed("move_forward"): move_dir.z -= 1
	if Input.is_action_pressed("move_back"): move_dir.z += 1
	if Input.is_action_pressed("move_left"): move_dir.x -= 1
	if Input.is_action_pressed("move_right"): move_dir.x += 1

	if move_dir.length() > 0 and camera:
		move_dir = move_dir.normalized()
		var speed = 30.0
		var forward = -camera.global_transform.basis.z.normalized()
		var right = camera.global_transform.basis.x.normalized()
		var movement = (forward * move_dir.z + right * move_dir.x) * speed * dt
		camera.global_translate(movement)

	if Input.is_action_pressed("jump") and camera:
		camera.global_translate(Vector3.UP * 15.0 * dt)

	if camera and Input.is_mouse_button_pressed(MOUSE_BUTTON_RIGHT):
		var mouse_delta = Input.get_last_mouse_velocity()
		camera.rotate_y(-mouse_delta.x * 0.003)
		camera.rotate_x(-mouse_delta.y * 0.003)
		camera.rotation.x = clamp(camera.rotation.x, -PI / 2, PI / 2)

func _input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP and camera:
			camera.global_translate(-camera.global_transform.basis.z * 5.0)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and camera:
			camera.global_translate(camera.global_transform.basis.z * 5.0)