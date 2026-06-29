extends Node3D
class_name InteractiveObject

@export var object_name: String = "Object"
@export var interaction_text: String = "Interact"
@export var pickupable: bool = false
@export var destructible: bool = false
@export var health: float = 100.0
@export var resource_type: String = ""
@export var resource_amount: int = 1

var wasteland_world
var highlighted: bool = false

func _ready():
	if destructible:
		add_to_group("destructible")
	if pickupable:
		add_to_group("pickupable")

func interact():
	print("[%s] Player interacts with %s" % [object_name, name])

	if pickupable:
		pickup()
	elif destructible:
		damage(10)

func pickup():
	if not pickupable:
		return
	print("[Item] Picked up %s" % object_name)
	queue_free()

func damage(amount: float):
	if not destructible:
		return

	health -= amount
	print("[Damage] %s took %.0f damage, health: %.0f" % [object_name, amount, health])

	if health <= 0:
		destroy()

func destroy():
	print("[Destroy] %s destroyed" % object_name)

	if resource_type and resource_amount > 0 and wasteland_world:
		var pos = global_position
		wasteland_world.spawn_iron_particles(pos.x, pos.y, pos.z, resource_amount, 0.5, false)
		if resource_type == "wood":
			wasteland_world.spawn_iron_particles(pos.x, pos.y + 1, pos.z, resource_amount / 2, 1.0, true)

	queue_free()

func setup_world(p_wasteland_world):
	wasteland_world = p_wasteland_world

func _on_mouse_entered():
	highlighted = true

func _on_mouse_exited():
	highlighted = false

class ResourceNode extends InteractiveObject:
	@export var respawn_time: float = 60.0
	@export var max_resources: int = 5
	var current_resources: int
	var respawning: bool = false

	func _ready():
		super._ready()
		current_resources = max_resources

	func interact():
		if current_resources > 0:
			current_resources -= 1
			print("[Resource] Gathered %s, %d remaining" % [resource_type, current_resources])
			if current_resources <= 0:
				_start_respawn()

	func _start_respawn():
		respawning = true
		visible = false
		var timer = get_tree().create_timer(respawn_time)
		timer.timeout.connect(_respawn)

	func _respawn():
		current_resources = max_resources
		respawning = false
		visible = true
		print("[Resource] %s respawned" % object_name)

class DestructibleBuilding extends InteractiveObject:
	@export var building_material: String = "concrete"
	@export var collapse_threshold: float = 30.0

	func _ready():
		super._ready()
		health = 200.0

	func destroy():
		var pos = global_position
		var bounds = Vector3(5, 5, 5)

		if wasteland_world:
			wasteland_world.spawn_voxel_grid(4, 4, 4, 2.0, pos.x - 4, pos.y, pos.z - 4)
			wasteland_world.apply_explosion(pos.x, pos.y + 3, pos.z, 8.0, 150.0)

		for i in range(randi_range(5, 15)):
			var debris = MeshInstance3D.new()
			debris.mesh = BoxMesh.new()
			debris.mesh.size = Vector3(
				randf_range(0.5, 2.0),
				randf_range(0.3, 1.0),
				randf_range(0.5, 2.0)
			)
			var mat = StandardMaterial3D.new()
			mat.albedo_color = Color(0.4, 0.35, 0.3)
			mat.roughness = 0.8
			debris.material_override = mat
			debris.position = pos + Vector3(
				randf_range(-5, 5),
				randf_range(0, 8),
				randf_range(-5, 5)
			)

			var body = RigidBody3D.new()
			var shape = CollisionShape3D.new()
			shape.shape = BoxShape3D.new()
			shape.shape.size = debris.mesh.size
			body.add_child(shape)
			body.add_child(debris)
			body.position = debris.position
			body.linear_velocity = Vector3(
				randf_range(-5, 5),
				randf_range(2, 10),
				randf_range(-5, 5)
			)
			get_parent().add_child(body)

		super.destroy()