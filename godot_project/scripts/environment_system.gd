extends Node3D
class_name EnvironmentSystem

enum Weather { CLEAR, CLOUDY, RAIN, STORM, SNOW, FOG, SANDSTORM, RADIATION }
enum TimeOfDay { DAWN, MORNING, NOON, AFTERNOON, DUSK, NIGHT }

@export var day_length_seconds: float = 600.0
@export var weather_transition_time: float = 30.0

var current_weather: Weather = Weather.CLEAR
var target_weather: Weather = Weather.CLEAR
var weather_blend: float = 1.0

var time_of_day: float = 0.3
var day_count: int = 1

var sun: DirectionalLight3D
var sky: WorldEnvironment
var rain_particles: GPUParticles3D
var fog_particles: GPUParticles3D

var _weather_node: Node
var _wind_direction: Vector3 = Vector3(1, 0, 0.5).normalized()
var _wind_speed: float = 0.0
var _temperature: float = 20.0
var _humidity: float = 0.5
var _radiation_level: float = 0.0
var _visibility: float = 1000.0

signal weather_changed(weather: Weather)
signal time_changed(time: float, period: TimeOfDay)
signal environment_updated(data: Dictionary)

const WEATHER_CONFIG = {
	Weather.CLEAR: {
		"sun_energy": 1.0,
		"sky_tint": Color(0.5, 0.7, 1.0),
		"ambient_energy": 0.5,
		"wind_speed": 2.0,
		"temperature_mod": 0.0,
		"humidity_mod": 0.0,
		"visibility": 1000.0,
		"rain_intensity": 0.0,
		"radiation_mod": 0.0,
	},
	Weather.CLOUDY: {
		"sun_energy": 0.5,
		"sky_tint": Color(0.6, 0.65, 0.7),
		"ambient_energy": 0.4,
		"wind_speed": 5.0,
		"temperature_mod": -3.0,
		"humidity_mod": 0.2,
		"visibility": 500.0,
		"rain_intensity": 0.0,
		"radiation_mod": 0.0,
	},
	Weather.RAIN: {
		"sun_energy": 0.2,
		"sky_tint": Color(0.4, 0.45, 0.5),
		"ambient_energy": 0.3,
		"wind_speed": 8.0,
		"temperature_mod": -5.0,
		"humidity_mod": 0.5,
		"visibility": 200.0,
		"rain_intensity": 1.0,
		"radiation_mod": 0.0,
	},
	Weather.STORM: {
		"sun_energy": 0.1,
		"sky_tint": Color(0.2, 0.2, 0.3),
		"ambient_energy": 0.2,
		"wind_speed": 20.0,
		"temperature_mod": -8.0,
		"humidity_mod": 0.8,
		"visibility": 80.0,
		"rain_intensity": 2.0,
		"radiation_mod": 0.0,
	},
	Weather.SNOW: {
		"sun_energy": 0.6,
		"sky_tint": Color(0.8, 0.85, 0.9),
		"ambient_energy": 0.6,
		"wind_speed": 3.0,
		"temperature_mod": -20.0,
		"humidity_mod": 0.3,
		"visibility": 150.0,
		"rain_intensity": 0.0,
		"radiation_mod": 0.0,
	},
	Weather.FOG: {
		"sun_energy": 0.3,
		"sky_tint": Color(0.7, 0.7, 0.7),
		"ambient_energy": 0.4,
		"wind_speed": 1.0,
		"temperature_mod": -2.0,
		"humidity_mod": 0.7,
		"visibility": 30.0,
		"rain_intensity": 0.0,
		"radiation_mod": 0.0,
	},
	Weather.SANDSTORM: {
		"sun_energy": 0.15,
		"sky_tint": Color(0.7, 0.6, 0.4),
		"ambient_energy": 0.3,
		"wind_speed": 30.0,
		"temperature_mod": 10.0,
		"humidity_mod": -0.3,
		"visibility": 20.0,
		"rain_intensity": 0.0,
		"radiation_mod": 0.0,
	},
	Weather.RADIATION: {
		"sun_energy": 0.4,
		"sky_tint": Color(0.5, 0.8, 0.4),
		"ambient_energy": 0.5,
		"wind_speed": 5.0,
		"temperature_mod": 5.0,
		"humidity_mod": 0.0,
		"visibility": 400.0,
		"rain_intensity": 0.0,
		"radiation_mod": 5.0,
	},
}

func _ready():
	_setup_sun()
	_setup_sky()
	_setup_particles()

func _setup_sun():
	sun = DirectionalLight3D.new()
	sun.name = "Sun"
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 200.0
	add_child(sun)

func _setup_sky():
	sky = WorldEnvironment.new()
	sky.name = "SkyEnvironment"

	var env = Environment.new()
	env.background_mode = Environment.BG_SKY
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.5
	env.fog_enabled = true
	env.fog_light_color = Color(0.5, 0.6, 0.7)
	env.fog_depth_begin = 50.0
	env.fog_depth_end = 500.0

	var sky_resource = Sky.new()
	var sky_mat = ProceduralSkyMaterial.new()
	sky_mat.sky_top_color = Color(0.3, 0.5, 0.8)
	sky_mat.sky_horizon_color = Color(0.6, 0.7, 0.8)
	sky_mat.ground_bottom_color = Color(0.2, 0.2, 0.2)
	sky_mat.ground_horizon_color = Color(0.4, 0.4, 0.4)
	sky_resource.sky_material = sky_mat

	env.sky = sky_resource
	sky.environment = env
	add_child(sky)

func _setup_particles():
	rain_particles = GPUParticles3D.new()
	rain_particles.name = "RainParticles"
	rain_particles.emitting = false
	rain_particles.amount = 5000
	rain_particles.lifetime = 1.0
	rain_particles.explosiveness = 0.0
	rain_particles.randomness = 0.5

	var rain_mat = ParticleProcessMaterial.new()
	rain_mat.gravity = Vector3(0, -20, 0)
	rain_particles.process_material = rain_mat
	add_child(rain_particles)

func _process(delta):
	_update_time(delta)
	_update_weather(delta)
	_update_sun()
	_update_environment()

	if Engine.get_process_frames() % 30 == 0:
		_push_to_rust_modules()

func _update_time(delta):
	time_of_day += delta / day_length_seconds
	if time_of_day >= 1.0:
		time_of_day -= 1.0
		day_count += 1

	var period = _get_time_period()
	time_changed.emit(time_of_day, period)

func _update_weather(delta):
	if weather_blend < 1.0:
		weather_blend = min(1.0, weather_blend + delta / weather_transition_time)
		if weather_blend >= 1.0:
			current_weather = target_weather

func _update_sun():
	var angle = time_of_day * TAU - PI / 2
	sun.rotation.x = angle

	var sun_height = sin(angle)
	var config = WEATHER_CONFIG[current_weather]

	if sun_height > 0:
		var energy = config["sun_energy"] * sun_height
		sun.light_energy = energy
		sun.visible = true
	else:
		sun.light_energy = 0.0
		sun.visible = false

	sun.rotation.y = _wind_direction.x * 0.3

func _update_environment():
	var config = WEATHER_CONFIG[current_weather]
	var env = sky.environment

	env.ambient_light_energy = config["ambient_energy"]
	env.fog_depth_end = config["visibility"]

	var sky_mat = env.sky.sky_material as ProceduralSkyMaterial
	if sky_mat:
		sky_mat.sky_top_color = config["sky_tint"]

	_wind_speed = config["wind_speed"]
	_temperature = 20.0 + config["temperature_mod"]
	_humidity = clamp(0.5 + config["humidity_mod"], 0.0, 1.0)
	_visibility = config["visibility"]
	_radiation_level = config["radiation_mod"]

	var rain_intensity = config["rain_intensity"]
	rain_particles.emitting = rain_intensity > 0
	if rain_intensity > 0:
		rain_particles.amount = int(5000 * rain_intensity)

func _push_to_rust_modules():
	var parent = get_parent()
	if not parent:
		return

	for child in parent.get_children():
		if child.name == "WastelandWorld" and child.has_method("set_weather"):
			child.set_weather(
				_temperature,
				_humidity,
				_wind_speed,
				_radiation_level,
				1.0 if current_weather == Weather.RAIN else 0.0,
			)
			break

		if child.name == "WastelandChemistrySystem" and child.has_method("set_environment"):
			child.set_environment(
				_temperature + 273.15,
				101.3,
				0.21,
				_humidity,
				7.0,
				_radiation_level,
			)
			break

func set_weather(weather: Weather):
	if weather == current_weather:
		return

	target_weather = weather
	weather_blend = 0.0
	weather_changed.emit(weather)

func set_random_weather():
	var weathers = Weather.values()
	var new_weather = weathers[randi() % weathers.size()]

	var weights = {
		Weather.CLEAR: 30,
		Weather.CLOUDY: 25,
		Weather.RAIN: 15,
		Weather.STORM: 5,
		Weather.SNOW: 5,
		Weather.FOG: 10,
		Weather.SANDSTORM: 5,
		Weather.RADIATION: 5,
	}

	var total_weight = 0
	for w in weathers:
		total_weight += weights.get(w, 1)

	var roll = randi() % total_weight
	var cumulative = 0
	for w in weathers:
		cumulative += weights.get(w, 1)
		if roll < cumulative:
			new_weather = w
			break

	set_weather(new_weather)

func set_time_of_day(t: float):
	time_of_day = clamp(t, 0.0, 1.0)

func _get_time_period() -> TimeOfDay:
	if time_of_day < 0.2:
		return TimeOfDay.DAWN
	elif time_of_day < 0.35:
		return TimeOfDay.MORNING
	elif time_of_day < 0.55:
		return TimeOfDay.NOON
	elif time_of_day < 0.7:
		return TimeOfDay.AFTERNOON
	elif time_of_day < 0.8:
		return TimeOfDay.DUSK
	else:
		return TimeOfDay.NIGHT

func get_environment_data() -> Dictionary:
	return {
		"weather": Weather.keys()[current_weather],
		"time_of_day": time_of_day,
		"day_count": day_count,
		"temperature": _temperature,
		"humidity": _humidity,
		"wind_speed": _wind_speed,
		"wind_direction": _wind_direction,
		"visibility": _visibility,
		"radiation_level": _radiation_level,
		"period": TimeOfDay.keys()[_get_time_period()],
	}
