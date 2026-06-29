extends Node3D
class_name AtmosphereController

var sun: DirectionalLight3D
var world_env: WorldEnvironment
var time_of_day: float = 10.0
var day_length: float = 600.0
var time_speed: float = 1.0
var paused_time: bool = false

var night_color: Color = Color(0.05, 0.05, 0.15)
var dawn_color: Color = Color(0.8, 0.6, 0.4)
var day_color: Color = Color(1.0, 0.95, 0.8)
var dusk_color: Color = Color(1.0, 0.5, 0.2)
var noon_color: Color = Color(1.0, 0.95, 0.8)

var fog_morning_density: float = 0.015
var fog_day_density: float = 0.005
var fog_evening_density: float = 0.01
var fog_night_density: float = 0.008

var cloud_threshold: float = 0.4
var rain_chance: float = 0.15
var is_raining: bool = false
var rain_intensity: float = 0.0
var rain_particles: GPUParticles3D

func _ready():
	_init_sun()
	_init_environment()
	_init_rain()
	print("[Atmosphere] Day/Night cycle active, day_length: %.0fs" % day_length)

func _init_sun():
	var found_sun = get_parent().get_node_or_null("DirectionalLight3D")
	if not found_sun:
		found_sun = get_tree().get_first_node_in_group("Sun")
	if found_sun and found_sun is DirectionalLight3D:
		sun = found_sun

func _init_environment():
	var found_env = get_parent().get_node_or_null("WorldEnv")
	if not found_env:
		found_env = get_tree().get_first_node_in_group("WorldEnv")
	if found_env and found_env is WorldEnvironment:
		world_env = found_env

func _init_rain():
	rain_particles = GPUParticles3D.new()
	rain_particles.name = "RainParticles"
	rain_particles.visible = false

	var particle_material = ParticleProcessMaterial.new()
	particle_material.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	particle_material.emission_box_extents = Vector3(300, 1, 300)
	particle_material.direction = Vector3(0, -1, 0)
	particle_material.spread = 5.0
	particle_material.gravity = Vector3(0, -30, 0)
	particle_material.initial_velocity_min = 20.0
	particle_material.initial_velocity_max = 40.0
	particle_material.lifetime = 2.0
	particle_material.scale_min = 0.02
	particle_material.scale_max = 0.05

	rain_particles.process_material = particle_material
	rain_particles.amount = 5000
	rain_particles.emitting = false

	var rain_mesh = BoxMesh.new()
	rain_mesh.size = Vector3(0.03, 0.3, 0.03)
	rain_particles.draw_pass_1 = rain_mesh

	var rain_mat = StandardMaterial3D.new()
	rain_mat.albedo_color = Color(0.6, 0.7, 0.9, 0.6)
	rain_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	rain_particles.material_override = rain_mat

	add_child(rain_particles)

func _process(delta):
	if paused_time:
		return

	time_of_day += delta * time_speed / day_length * 24.0
	if time_of_day >= 24.0:
		time_of_day -= 24.0

	_update_sun()
	_update_sky()
	_update_fog(delta)
	_update_rain(delta)
	_update_ambient()

func _update_sun():
	if not sun:
		return

	var sun_angle = (time_of_day - 6.0) / 24.0 * TAU
	sun_angle = clamp(sun_angle, -PI / 2, PI / 2)

	sun.rotation = Vector3(sun_angle, 0.5, 0)

	var t = abs(sun_angle) / (PI / 2)
	if time_of_day < 6 or time_of_day > 18:
		var night_t = clamp((min(abs(time_of_day - 24), abs(time_of_day)) / 3.0), 0.0, 1.0)
		sun.light_color = night_color.lerp(dawn_color, 1.0 - night_t)
		sun.light_energy = 0.1 + (1.0 - night_t) * 1.4
	else:
		var day_t = clamp(abs(time_of_day - 12.0) / 6.0, 0.0, 1.0)
		sun.light_color = noon_color.lerp(dusk_color, day_t)
		sun.light_energy = 1.5 - day_t * 0.5

func _update_sky():
	if not world_env or not world_env.environment:
		return

	if not world_env.environment.sky:
		return

	var sky_mat = world_env.environment.sky.sky_material
	if not sky_mat or not sky_mat is ProceduralSkyMaterial:
		return

	if time_of_day < 5 or time_of_day > 20:
		sky_mat.sky_top_color = Color(0.02, 0.02, 0.1)
		sky_mat.sky_horizon_color = Color(0.05, 0.05, 0.15)
	elif time_of_day < 7:
		var t = (time_of_day - 5) / 2.0
		sky_mat.sky_top_color = Color(0.02, 0.05, 0.2).lerp(Color(0.3, 0.5, 0.8), t)
		sky_mat.sky_horizon_color = Color(0.5, 0.3, 0.2).lerp(Color(0.8, 0.7, 0.5), t)
	elif time_of_day < 17:
		sky_mat.sky_top_color = Color(0.2, 0.4, 0.8)
		sky_mat.sky_horizon_color = Color(0.7, 0.8, 0.9)
	elif time_of_day < 19:
		var t = (time_of_day - 17) / 2.0
		sky_mat.sky_top_color = Color(0.2, 0.4, 0.8).lerp(Color(0.05, 0.02, 0.1), t)
		sky_mat.sky_horizon_color = Color(0.8, 0.5, 0.3).lerp(Color(0.1, 0.05, 0.2), t)
	else:
		var t = (time_of_day - 19) / 1.0
		sky_mat.sky_top_color = Color(0.02, 0.02, 0.1)
		sky_mat.sky_horizon_color = Color(0.5, 0.3, 0.2).lerp(Color(0.05, 0.05, 0.15), t)

func _update_fog(delta):
	if not world_env or not world_env.environment:
		return

	var target_density = fog_day_density
	if time_of_day < 6:
		var t = 1.0 - clamp(time_of_day / 4.0, 0.0, 1.0)
		target_density = lerp(fog_morning_density, fog_night_density, t)
	elif time_of_day < 8:
		target_density = lerp(fog_morning_density, fog_day_density, (time_of_day - 6) / 2.0)
	elif time_of_day < 16:
		target_density = fog_day_density
	elif time_of_day < 19:
		target_density = lerp(fog_day_density, fog_evening_density, (time_of_day - 16) / 3.0)
	elif time_of_day < 21:
		target_density = lerp(fog_evening_density, fog_night_density, (time_of_day - 19) / 2.0)
	else:
		target_density = fog_night_density

	if is_raining:
		target_density *= 1.5

	var current = world_env.environment.volumetric_fog_density
	world_env.environment.volumetric_fog_density = lerp(current, target_density, delta * 2.0)

func _update_rain(delta):
	var prev_rain = is_raining

	if is_raining:
		rain_intensity = max(rain_intensity - delta * 0.02, 0.0)
		if rain_intensity <= 0:
			is_raining = false
			rain_intensity = 0.0
	else:
		if randf() < rain_chance * delta * 0.05:
			is_raining = true
			rain_intensity = randf_range(0.3, 1.0)

	if is_raining != prev_rain:
		if is_raining:
			rain_particles.visible = true
			rain_particles.emitting = true
			rain_particles.amount = int(rain_intensity * 5000)
			if world_env and world_env.environment:
				world_env.environment.glow_enabled = true
				world_env.environment.glow_bloom = 0.15
				world_env.environment.adjustment_saturation = 0.85
		else:
			rain_particles.emitting = false
			rain_particles.visible = false
			if world_env and world_env.environment:
				world_env.environment.glow_enabled = true
				world_env.environment.glow_bloom = 0.1
				world_env.environment.adjustment_saturation = 1.05

func _update_ambient():
	var ambient_energy: float
	if time_of_day < 6:
		ambient_energy = 0.1
	elif time_of_day < 8:
		ambient_energy = lerp(0.2, 0.6, (time_of_day - 6) / 2.0)
	elif time_of_day < 16:
		ambient_energy = 0.7
	elif time_of_day < 19:
		ambient_energy = lerp(0.7, 0.2, (time_of_day - 16) / 3.0)
	else:
		ambient_energy = 0.15

	if is_raining:
		ambient_energy *= 0.6

	if get_viewport() and get_viewport().world_3d:
		var env = get_viewport().world_3d.environment
		if env:
			env.ambient_light_energy = ambient_energy

func set_time(hour: float):
	time_of_day = clamp(hour, 0.0, 24.0)

func get_time() -> float:
	return time_of_day

func is_night() -> bool:
	return time_of_day < 6 or time_of_day > 18

func is_daytime() -> bool:
	return time_of_day >= 6 and time_of_day <= 18