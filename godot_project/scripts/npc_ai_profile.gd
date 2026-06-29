extends Node3D
class_name WastelandAIProfile

@export var character_name: String
@export var species: String = "human"
@export var faction: String = "wanderers"
@export var personality_traits: Array[String] = []

var npc_id: String
var wasteland_world
var known_locations: Array[Vector3] = []
var memory: Dictionary = {}
var current_goal: String = ""
var goal_progress: float = 0.0

func _ready():
	if species == "animal":
		_setup_animal()
	else:
		_setup_humanoid()

func _setup_humanoid():
	var body = $MeshInstance3D
	if body:
		var body_mat = StandardMaterial3D.new()
		var skin_tone = randf()
		if skin_tone < 0.4:
			body_mat.albedo_color = Color(0.85, 0.7, 0.55)
		elif skin_tone < 0.7:
			body_mat.albedo_color = Color(0.6, 0.4, 0.25)
		else:
			body_mat.albedo_color = Color(0.95, 0.8, 0.65)
		body_mat.roughness = 0.6
		body_mat.metallic = 0.0
		body.material_override = body_mat

	var hair_chance = randf_range(0.3, 3.0)
	var head_node = $Head if has_node("Head") else null
	if head_node and hair_chance > 0.3:
		var hair = MeshInstance3D.new()
		hair.mesh = SphereMesh.new()
		hair.mesh.radius = 0.1
		hair.mesh.height = 0.3
		var hair_mat = StandardMaterial3D.new()
		var hair_colors = [
			Color(0.1, 0.05, 0.02),
			Color(0.3, 0.2, 0.1),
			Color(0.15, 0.08, 0.03),
			Color(0.7, 0.5, 0.2),
			Color(0.9, 0.7, 0.3)
		]
		hair_mat.albedo_color = hair_colors[randi() % hair_colors.size()]
		hair.material_override = hair_mat
		hair.position = Vector3(0, 0.15, 0)
		head_node.add_child(hair)

func _setup_animal():
	var body_mesh = SphereMesh.new()
	body_mesh.radius = 0.6
	body_mesh.height = 1.2

	var body = $MeshInstance3D
	if body:
		body.mesh = body_mesh
		var body_mat = StandardMaterial3D.new()
		var animal_colors = [
			Color(0.3, 0.2, 0.1),
			Color(0.4, 0.3, 0.2),
			Color(0.25, 0.15, 0.08),
			Color(0.5, 0.35, 0.2)
		]
		body_mat.albedo_color = animal_colors[randi() % animal_colors.size()]
		body_mat.roughness = 0.7
		body.material_override = body_mat
		body.scale = Vector3(1.0, 0.6, 1.3)

	var head = MeshInstance3D.new()
	head.mesh = SphereMesh.new()
	head.mesh.radius = 0.25
	head.mesh.height = 0.3
	head.position = Vector3(0, 0.6, 0.5)
	var head_mat = StandardMaterial3D.new()
	head_mat.albedo_color = Color(0.35, 0.25, 0.15)
	head.material_override = head_mat
	add_child(head)

func setup_world(p_wasteland_world):
	wasteland_world = p_wasteland_world

func spawn_in_world():
	if not wasteland_world:
		return

	npc_id = wasteland_world.spawn_npc(
		character_name,
		global_position.x, global_position.y, global_position.z,
		species, faction
	)

	if npc_id:
		print("[AI] %s spawned with id %s" % [character_name, npc_id])

func say_dialogue(player_message: String):
	if not wasteland_world or not npc_id:
		return null

	var response = wasteland_world.npc_dialogue(npc_id, player_message)
	if response and response.has("text"):
		print("[%s]: %s" % [character_name, response["text"]])
	return response

func set_goal(goal: String):
	current_goal = goal
	goal_progress = 0.0

func update_goal(progress_delta: float):
	goal_progress = min(goal_progress + progress_delta, 1.0)
	if goal_progress >= 1.0 and current_goal:
		print("[AI] %s completed goal: %s" % [character_name, current_goal])
		current_goal = ""