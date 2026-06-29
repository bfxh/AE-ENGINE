extends Node3D
class_name InteractiveObject

enum InteractionType {
	PICKUP, CONTAINER, DOOR, LEVER, TERMINAL,
	DESTROYABLE, RESOURCE_NODE, TRAP, CRAFTING_STATION,
}

@export var interaction_type: InteractionType = InteractionType.PICKUP
@export var interaction_label: String = "Interact"
@export var required_tool: String = ""
@export var is_locked: bool = false
@export var lock_difficulty: int = 1
@export var health: float = 100.0
@export var max_health: float = 100.0
@export var respawn_time: float = 300.0
@export var destroy_particles: bool = true
@export var destroy_sound: bool = true

var _is_destroyed: bool = false
var _respawn_timer: float = 0.0
var _interaction_cooldown: float = 0.0
var _highlight_material: StandardMaterial3D
var _original_materials: Dictionary = {}

signal interacted(character: Node3D)
signal destroyed(by: Node3D)
signal respawned()

func _ready():
	if health <= 0:
		health = max_health
	_setup_highlight()

func _process(delta: float):
	if _is_destroyed and respawn_time > 0:
		_respawn_timer += delta
		if _respawn_timer >= respawn_time:
			respawn()

	if _interaction_cooldown > 0:
		_interaction_cooldown -= delta

func interact(character: Node3D) -> bool:
	if _is_destroyed:
		return false
	if _interaction_cooldown > 0:
		return false

	match interaction_type:
		InteractionType.PICKUP:
			return _interact_pickup(character)
		InteractionType.CONTAINER:
			return _interact_container(character)
		InteractionType.DOOR:
			return _interact_door(character)
		InteractionType.LEVER:
			return _interact_lever(character)
		InteractionType.TERMINAL:
			return _interact_terminal(character)
		InteractionType.DESTROYABLE:
			return _interact_destroyable(character)
		InteractionType.RESOURCE_NODE:
			return _interact_resource(character)
		InteractionType.TRAP:
			return _interact_trap(character)
		InteractionType.CRAFTING_STATION:
			return _interact_crafting(character)

	return false

func _interact_pickup(character: Node3D) -> bool:
	print("[Pickup] %s picked up %s" % [character.name, name])
	interacted.emit(character)
	queue_free()
	return true

func _interact_container(character: Node3D) -> bool:
	if is_locked:
		if character.has_method("try_unlock"):
			if character.try_unlock(lock_difficulty):
				is_locked = false
				print("[Container] Unlocked by %s" % character.name)
			else:
				print("[Container] Lock picking failed")
				return false

	var loot = _generate_loot()
	print("[Container] Opened, contains: ", loot.size(), " items")
	interacted.emit(character)
	_interaction_cooldown = 2.0
	return true

func _interact_door(character: Node3D) -> bool:
	if is_locked:
		if character.has_method("has_key"):
			if character.has_key(name):
				is_locked = false
			else:
				return false

	var target_rotation = rotation_degrees.y + 90
	var tween = create_tween()
	tween.tween_property(self, "rotation_degrees:y", target_rotation, 0.5)
	interacted.emit(character)
	return true

func _interact_lever(character: Node3D) -> bool:
	var target_rotation = rotation_degrees.x + 45
	var tween = create_tween()
	tween.tween_property(self, "rotation_degrees:x", target_rotation, 0.3)
	interacted.emit(character)
	return true

func _interact_terminal(character: Node3D) -> bool:
	print("[Terminal] Activated by %s" % character.name)
	interacted.emit(character)
	return true

func _interact_destroyable(character: Node3D) -> bool:
	take_damage(25, character)
	interacted.emit(character)
	return true

func _interact_resource(character: Node3D) -> bool:
	if required_tool != "":
		if not character.has_method("has_tool") or not character.has_tool(required_tool):
			print("[Resource] Need tool: %s" % required_tool)
			return false

	var resources = _generate_resources()
	print("[Resource] Gathered: ", resources)
	interacted.emit(character)
	_interaction_cooldown = 3.0
	return true

func _interact_trap(character: Node3D) -> bool:
	if character.has_method("take_damage"):
		var damage = randi_range(20, 50)
		character.take_damage(damage)
		print("[Trap] Dealt %d damage to %s" % [damage, character.name])
	interacted.emit(character)
	return true

func _interact_crafting(character: Node3D) -> bool:
	print("[Crafting] Station activated by %s" % character.name)
	interacted.emit(character)
	return true

func take_damage(amount: float, source: Node = null):
	if _is_destroyed:
		return

	health -= amount
	if health <= 0:
		health = 0
		_destroy(source)

func _destroy(source: Node = null):
	_is_destroyed = true
	_respawn_timer = 0.0

	if destroy_particles:
		_spawn_destroy_effects()

	if destroy_sound:
		_play_destroy_sound()

	visible = false
	set_process(true)
	destroyed.emit(source)
	print("[Destroy] %s destroyed by %s" % [name, source.name if source else "unknown"])

func respawn():
	_is_destroyed = false
	health = max_health
	visible = true
	_respawn_timer = 0.0
	respawned.emit()
	print("[Respawn] %s has respawned" % name)

func _generate_loot() -> Array:
	var loot_table = [
		{"name": "scrap_metal", "count": randi_range(1, 5), "weight": 0.4},
		{"name": "cloth", "count": randi_range(1, 3), "weight": 0.3},
		{"name": "water_bottle", "count": 1, "weight": 0.2},
		{"name": "ammo_9mm", "count": randi_range(5, 15), "weight": 0.15},
		{"name": "medkit", "count": 1, "weight": 0.05},
		{"name": "weapon_parts", "count": randi_range(1, 2), "weight": 0.1},
	]

	var loot = []
	for item in loot_table:
		if randf() < item["weight"]:
			loot.append({"name": item["name"], "count": item["count"]})
	return loot

func _generate_resources() -> Array:
	var resource_types = {
		"tree": [{"name": "wood", "count": randi_range(3, 8)}, {"name": "sap", "count": randi_range(0, 2)}],
		"rock": [{"name": "stone", "count": randi_range(2, 5)}, {"name": "iron_ore", "count": randi_range(0, 2)}],
		"scrap": [{"name": "scrap_metal", "count": randi_range(2, 6)}, {"name": "copper_wire", "count": randi_range(0, 3)}],
		"plant": [{"name": "fiber", "count": randi_range(2, 5)}, {"name": "herb", "count": randi_range(0, 3)}],
	}

	var type_key = "scrap"
	for key in resource_types:
		if name.to_lower().find(key) != -1:
			type_key = key
			break

	return resource_types[type_key] if resource_types.has(type_key) else resource_types["scrap"]

func _spawn_destroy_effects():
	var particles = CPUParticles3D.new()
	particles.emitting = true
	particles.amount = 15
	particles.lifetime = 1.5
	particles.explosiveness = 1.0
	particles.gravity = Vector3(0, -9.8, 0)
	particles.position = global_position
	particles.one_shot = true

	var particle_material = StandardMaterial3D.new()
	particle_material.albedo_color = Color(0.6, 0.3, 0.1) if randi() % 2 == 0 else Color(0.3, 0.3, 0.3)
	particle_material.roughness = 0.9

	get_parent().add_child(particles)
	var timer = get_tree().create_timer(2.0)
	timer.timeout.connect(particles.queue_free)

func _play_destroy_sound():
	pass

func _setup_highlight():
	_highlight_material = StandardMaterial3D.new()
	_highlight_material.albedo_color = Color(1.0, 0.9, 0.2, 0.5)
	_highlight_material.emission_enabled = true
	_highlight_material.emission = Color(0.3, 0.3, 0.1)

func highlight(active: bool):
	for child in get_children():
		if child is MeshInstance3D:
			if active:
				if not _original_materials.has(child):
					_original_materials[child] = child.material_override
				child.material_override = _highlight_material
			else:
				if _original_materials.has(child):
					child.material_override = _original_materials[child]

func get_interaction_data() -> Dictionary:
	return {
		"name": name,
		"type": interaction_type,
		"label": interaction_label,
		"locked": is_locked,
		"health": health,
		"max_health": max_health,
		"destroyed": _is_destroyed,
	}