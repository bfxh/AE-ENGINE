extends CanvasLayer
class_name GameHUD

var health_bar: ProgressBar
var stamina_bar: ProgressBar
var hunger_bar: ProgressBar
var thirst_bar: ProgressBar
var radiation_bar: ProgressBar

var health_label: Label
var stamina_label: Label
var status_label: Label
var inventory_label: Label
var crosshair: CenterContainer
var interaction_prompt: Label
var debug_panel: Panel

var _player: PlayerController
var _inventory: InventorySystem
var _environment: EnvironmentSystem

var _show_debug: bool = false

signal inventory_toggle_requested()

func _ready():
	layer = 10
	_setup_hud()

func _setup_hud():
	var margin = 20

	var bars_container = VBoxContainer.new()
	bars_container.name = "BarsContainer"
	bars_container.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	bars_container.position = Vector2(margin, -180)
	bars_container.size = Vector2(200, 160)
	add_child(bars_container)

	health_bar = _create_bar("Health", Color(0.8, 0.2, 0.2), bars_container)
	stamina_bar = _create_bar("Stamina", Color(0.2, 0.7, 0.2), bars_container)
	hunger_bar = _create_bar("Hunger", Color(0.8, 0.6, 0.2), bars_container)
	thirst_bar = _create_bar("Thirst", Color(0.2, 0.4, 0.8), bars_container)
	radiation_bar = _create_bar("Radiation", Color(0.6, 0.8, 0.2), bars_container)

	health_label = Label.new()
	health_label.name = "HealthLabel"
	health_label.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	health_label.position = Vector2(margin, -200)
	health_label.size = Vector2(200, 20)
	add_child(health_label)

	status_label = Label.new()
	status_label.name = "StatusLabel"
	status_label.set_anchors_preset(Control.PRESET_TOP_LEFT)
	status_label.position = Vector2(margin, margin)
	status_label.size = Vector2(300, 60)
	status_label.add_theme_font_size_override("font_size", 14)
	add_child(status_label)

	inventory_label = Label.new()
	inventory_label.name = "InventoryLabel"
	inventory_label.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	inventory_label.position = Vector2(-220, margin)
	inventory_label.size = Vector2(200, 30)
	inventory_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	inventory_label.add_theme_font_size_override("font_size", 12)
	add_child(inventory_label)

	crosshair = CenterContainer.new()
	crosshair.name = "Crosshair"
	crosshair.set_anchors_preset(Control.PRESET_CENTER)
	crosshair.size = Vector2(20, 20)

	var cross_dot = ColorRect.new()
	cross_dot.size = Vector2(4, 4)
	cross_dot.color = Color(1, 1, 1, 0.7)
	crosshair.add_child(cross_dot)
	add_child(crosshair)

	interaction_prompt = Label.new()
	interaction_prompt.name = "InteractionPrompt"
	interaction_prompt.set_anchors_preset(Control.PRESET_CENTER)
	interaction_prompt.position = Vector2(-100, 30)
	interaction_prompt.size = Vector2(200, 30)
	interaction_prompt.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	interaction_prompt.add_theme_font_size_override("font_size", 16)
	interaction_prompt.visible = false
	add_child(interaction_prompt)

	debug_panel = Panel.new()
	debug_panel.name = "DebugPanel"
	debug_panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	debug_panel.position = Vector2(-320, 60)
	debug_panel.size = Vector2(300, 250)
	debug_panel.visible = false
	add_child(debug_panel)

	var debug_label = Label.new()
	debug_label.name = "DebugLabel"
	debug_label.position = Vector2(10, 10)
	debug_label.size = Vector2(280, 230)
	debug_label.add_theme_font_size_override("font_size", 11)
	debug_panel.add_child(debug_label)

func _create_bar(name: String, color: Color, parent: VBoxContainer) -> ProgressBar:
	var container = HBoxContainer.new()
	container.name = name + "Container"

	var label = Label.new()
	label.text = name.left(3) + ":"
	label.custom_minimum_size = Vector2(35, 0)
	label.add_theme_font_size_override("font_size", 10)
	container.add_child(label)

	var bar = ProgressBar.new()
	bar.name = name
	bar.min_value = 0
	bar.max_value = 100
	bar.value = 100
	bar.custom_minimum_size = Vector2(150, 12)
	bar.show_percentage = false

	var style_bg = StyleBoxFlat.new()
	style_bg.bg_color = Color(0.1, 0.1, 0.1, 0.7)
	bar.add_theme_stylebox_override("background", style_bg)

	var style_fill = StyleBoxFlat.new()
	style_fill.bg_color = color
	bar.add_theme_stylebox_override("fill", style_fill)

	container.add_child(bar)
	parent.add_child(container)

	return bar

func setup(player: PlayerController, inventory: InventorySystem, environment: EnvironmentSystem):
	_player = player
	_inventory = inventory
	_environment = environment

	if _player:
		_player.debug_info_requested.connect(_toggle_debug)
		_player.interact_pressed.connect(_on_interact)

func _process(_delta):
	if _player and _player.movement:
		stamina_bar.value = _player.movement.stamina
		health_label.text = "HP: 100 | %s | %.1f m/s" % [
			_player.movement.get_state_name(),
			_player.movement.velocity.length()
		]

	if _inventory:
		inventory_label.text = "Items: %d/%d | %.1f/%.1f kg" % [
			_inventory.get_used_slots(),
			_inventory._max_slots,
			_inventory.get_current_weight(),
			_inventory.get_max_weight()
		]

	if _environment:
		var data = _environment.get_environment_data()
		status_label.text = "Day %d | %s | %.0f°C | %s" % [
			data["day_count"],
			data["period"],
			data["temperature"],
			data["weather"],
		]

	if _show_debug and _player:
		var debug_label = debug_panel.get_node("DebugLabel")
		if debug_label:
			var info = _player.get_debug_info()
			debug_label.text = "=== DEBUG ===\n"
			debug_label.text += "State: %s\n" % info.get("state", "?")
			debug_label.text += "Position: %.1f, %.1f, %.1f\n" % [info["position"].x, info["position"].y, info["position"].z]
			debug_label.text += "Velocity: %.2f\n" % info.get("speed", 0)
			debug_label.text += "Grounded: %s\n" % info.get("grounded", false)
			debug_label.text += "Stamina: %.1f\n" % info.get("stamina", 0)
			debug_label.text += "Yaw: %.2f | Pitch: %.2f\n" % [info.get("camera_yaw", 0), info.get("camera_pitch", 0)]

			if _environment:
				var env = _environment.get_environment_data()
				debug_label.text += "Weather: %s\n" % env["weather"]
				debug_label.text += "Wind: %.1f m/s\n" % env["wind_speed"]
				debug_label.text += "Radiation: %.1f\n" % env["radiation_level"]

	if _player and _player.interaction_ray:
		if _player.interaction_ray.is_colliding():
			var target = _player._find_interactable_parent(_player.interaction_ray.get_collider())
			if target:
				interaction_prompt.text = "[E] Interact"
				interaction_prompt.visible = true
			else:
				interaction_prompt.visible = false
		else:
			interaction_prompt.visible = false

func _toggle_debug():
	_show_debug = not _show_debug
	debug_panel.visible = _show_debug

func _on_interact(target: Node3D):
	if target and target.has_node("InteractionData"):
		var data = target.get_node("InteractionData")
		var type = data.get_meta("interaction_type", "examine")
		status_label.text = "Interacted: %s (%s)" % [target.name, type]

func show_message(text: String, duration: float = 3.0):
	status_label.text = text
	var timer = get_tree().create_timer(duration)
	timer.timeout.connect(func(): if status_label.text == text: status_label.text = "")
