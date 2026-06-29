extends Node3D
## MPSS Particle System Demo
## Controls: P=toggle, Click=spawn grid, 1=fluid (500), 2=eruption (2000), 3=clear

var mpss_node: Node = null
var multi_mesh: MultiMeshInstance3D = null
var ground: MeshInstance3D = null
var particles_visible: bool = true
var last_count: int = -1
var frame_count: int = 0

func _ready():
	for child in get_children():
		if child is Node3D and child.has_method("spawn_grid"):
			mpss_node = child
			break

	if not mpss_node:
		push_error("[MPSS Demo] No WastelandMpss node found!")
		return

	# Ground plane
	ground = MeshInstance3D.new()
	ground.name = "Ground"
	var gm = PlaneMesh.new()
	gm.size = Vector2(30, 30)
	ground.mesh = gm
	var gmat = StandardMaterial3D.new()
	gmat.albedo_color = Color(0.12, 0.12, 0.14)
	ground.material_override = gmat
	add_child(ground)

	# MultiMesh - 0.1m emissive cubes
	multi_mesh = MultiMeshInstance3D.new()
	multi_mesh.name = "Particles"
	var mm = MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	var cube = BoxMesh.new()
	cube.size = Vector3(0.1, 0.1, 0.1)
	mm.mesh = cube
	mm.instance_count = 0
	multi_mesh.multimesh = mm

	var mat = StandardMaterial3D.new()
	mat.albedo_color = Color(1.0, 0.35, 0.05)
	mat.emission_enabled = true
	mat.emission = Color(1.0, 0.3, 0.0)
	mat.emission_energy_multiplier = 0.8
	multi_mesh.material_override = mat
	add_child(multi_mesh)

	# Auto-spawn
	mpss_node.spawn_grid(10, 0.5)
	print("[MPSS Demo] Init: ", mpss_node.particle_count, " particles")

func _process(_delta):
	if not mpss_node:
		return

	frame_count += 1
	var cur = mpss_node.particle_count
	var positions = mpss_node.get_positions()
	var n = positions.size()

	# Sync instance count (handles both increase and decrease)
	if multi_mesh.multimesh.instance_count != n:
		multi_mesh.multimesh.instance_count = n
		if frame_count < 5 or cur != last_count:
			print("[MPSS Demo] Frame ", frame_count, ": ", n, " particles")
		last_count = n

	# Update transforms
	multi_mesh.visible = particles_visible and n > 0
	for i in range(min(n, multi_mesh.multimesh.instance_count)):
		multi_mesh.multimesh.set_instance_transform(i, Transform3D(Basis(), positions[i]))

func _input(event):
	if Input.is_action_just_pressed("toggle_pause"):
		particles_visible = not particles_visible
		if multi_mesh:
			multi_mesh.visible = particles_visible
		print("[MPSS Demo] visible=", particles_visible)

	if Input.is_action_just_pressed("explode"):
		mpss_node.spawn_grid(10, 0.5)
		last_count = -1
		print("[MPSS Demo] Grid respawned: ", mpss_node.particle_count)

	if Input.is_action_just_pressed("action_npc_spawn"):
		mpss_node.spawn_fluid_column(500, 5.0, 2.0)
		last_count = -1
		print("[MPSS Demo] Fluid column: ", mpss_node.particle_count)

	if Input.is_action_just_pressed("action_explosion"):
		mpss_node.spawn_fluid_column(2000, 8.0, 4.0)
		last_count = -1
		print("[MPSS Demo] Eruption: ", mpss_node.particle_count)

	if Input.is_action_just_pressed("action_spawn_animals"):
		mpss_node.clear()
		last_count = -1
		print("[MPSS Demo] Cleared")