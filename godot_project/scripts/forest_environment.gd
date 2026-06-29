extends Node
class_name ForestEnvironment

var env_node
var terrain
var tree_system
var vegetation
var water_system
var atmosphere
var building_gen
var animal_spawner

var rng = RandomNumberGenerator.new()
var world_seed: int = 42

var biomes = {
	"dense_forest": {"tree_density": 0.7, "undergrowth": 0.8, "water_prob": 0.1, "altitude_min": 0, "altitude_max": 30},
	"sparse_woods": {"tree_density": 0.3, "undergrowth": 0.4, "water_prob": 0.05, "altitude_min": 10, "altitude_max": 50},
	"meadow": {"tree_density": 0.05, "undergrowth": 0.9, "water_prob": 0.02, "altitude_min": 5, "altitude_max": 20},
	"wetland": {"tree_density": 0.1, "undergrowth": 0.6, "water_prob": 0.5, "altitude_min": -5, "altitude_max": 5},
	"rocky_hill": {"tree_density": 0.15, "undergrowth": 0.2, "water_prob": 0.01, "altitude_min": 30, "altitude_max": 80},
	"ruins": {"tree_density": 0.1, "undergrowth": 0.3, "water_prob": 0.05, "altitude_min": 0, "altitude_max": 40},
}

var world_bounds = {
	"min_x": -500, "max_x": 500,
	"min_z": -500, "max_z": 500,
	"cell_size": 50,
}

var spawned_trees = []
var spawned_animals = []
var spawned_buildings = []

func _init(p_world_seed: int = 42):
	world_seed = p_world_seed
	rng.seed = world_seed

func setup(parent: Node):
	env_node = WorldEnvironment.new()
	env_node.name = "ForestEnvironment"
	parent.add_child(env_node)

	atmosphere = AtmosphereController.new()
	atmosphere.name = "Atmosphere"
	parent.add_child(atmosphere)

	terrain = TerrainGenerator.new()
	terrain.name = "Terrain"
	parent.add_child(terrain)

	tree_system = AdvancedTreeSystem.new()
	tree_system.name = "TreeSystem"
	parent.add_child(tree_system)

	vegetation = VegetationSpawner.new()
	vegetation.name = "Vegetation"
	parent.add_child(vegetation)

	water_system = WaterBodySystem.new()
	water_system.name = "WaterSystem"
	parent.add_child(water_system)

	building_gen = BuildingGenerator.new()
	building_gen.name = "BuildingGenerator"
	parent.add_child(building_gen)

	animal_spawner = AnimalSpawner.new()
	animal_spawner.name = "AnimalSpawner"
	parent.add_child(animal_spawner)

	print("[Forest] Environment systems initialized")

func generate_full_world():
	print("[Forest] Generating full forest world...")
	var start_time = Time.get_ticks_msec()

	terrain.generate_heightmap(world_bounds, world_seed)
	print("[Forest] Terrain heightmap generated")

	_generate_biome_map()
	print("[Forest] Biome map generated")

	place_trees()
	print("[Forest] Trees placed: ", spawned_trees.size())

	vegetation.spawn_vegetation(world_bounds, rng)
	print("[Forest] Vegetation spawned")

	water_system.generate_water_bodies(world_bounds, rng)
	print("[Forest] Water bodies generated")

	place_buildings()
	print("[Forest] Buildings placed: ", spawned_buildings.size())

	spawn_wildlife()
	print("[Forest] Animals spawned: ", spawned_animals.size())

	var elapsed = Time.get_ticks_msec() - start_time
	print("[Forest] World generation complete in %.1f seconds" % (elapsed / 1000.0))

func _generate_biome_map():
	var biome_grid = {}
	for cx in range(world_bounds.min_x, world_bounds.max_x, world_bounds.cell_size):
		for cz in range(world_bounds.min_z, world_bounds.max_z, world_bounds.cell_size):
			var altitude = terrain.get_height_at(cx, cz) if terrain else rng.randf_range(-10, 60)
			var key = "%d_%d" % [cx / world_bounds.cell_size, cz / world_bounds.cell_size]

			var selected_biome = "sparse_woods"
			for biome_name in biomes:
				var cfg = biomes[biome_name]
				if altitude >= cfg.altitude_min and altitude <= cfg.altitude_max:
					if rng.randf() < 0.4:
						selected_biome = biome_name
						break

			biome_grid[key] = selected_biome

	return biome_grid

func place_trees():
	spawned_trees.clear()

	for cx in range(world_bounds.min_x, world_bounds.max_x, 20):
		for cz in range(world_bounds.min_z, world_bounds.max_z, 20):
			var key = "%d_%d" % [int(cx / world_bounds.cell_size), int(cz / world_bounds.cell_size)]
			var altitude = terrain.get_height_at(cx, cz) if terrain else rng.randf_range(0, 30)

			var spawn_chance = 0.35
			if altitude < -3:
				spawn_chance = 0.05
			elif altitude > 50:
				spawn_chance = 0.1

			if rng.randf() > spawn_chance:
				continue

			var offset_x = rng.randf_range(-8, 8)
			var offset_z = rng.randf_range(-8, 8)
			var pos = Vector3(cx + offset_x, altitude, cz + offset_z)

			var species
			var roll = rng.randf()
			if roll < 0.35:
				species = tree_system.TreeSpecies.PINE
			elif roll < 0.55:
				species = tree_system.TreeSpecies.OAK
			elif roll < 0.7:
				species = tree_system.TreeSpecies.BIRCH
			elif roll < 0.8:
				species = tree_system.TreeSpecies.WILLOW
			elif roll < 0.9:
				species = tree_system.TreeSpecies.DEAD
			else:
				species = tree_system.TreeSpecies.BURNT

			var tree = tree_system.create_tree(species, pos, rng)
			if tree:
				spawned_trees.append({"position": pos, "species": species})

func place_buildings():
	spawned_buildings.clear()

	var ruin_clusters = rng.randi_range(3, 8)
	for i in range(ruin_clusters):
		var cx = rng.randf_range(world_bounds.min_x * 0.6, world_bounds.max_x * 0.6)
		var cz = rng.randf_range(world_bounds.min_z * 0.6, world_bounds.max_z * 0.6)
		var altitude = terrain.get_height_at(cx, cz) if terrain else 0

		var building_count = rng.randi_range(1, 4)
		for j in range(building_count):
			var bx = cx + rng.randf_range(-30, 30)
			var bz = cz + rng.randf_range(-30, 30)
			var building_type = rng.randi() % 4

			match building_type:
				0:
					building_gen.generate_ruined_house(Vector3(bx, altitude, bz), rng)
				1:
					building_gen.generate_collapsed_tower(Vector3(bx, altitude, bz), rng)
				2:
					building_gen.generate_bunker_entrance(Vector3(bx, altitude, bz), rng)
				3:
					building_gen.generate_scrap_wall(Vector3(bx, altitude, bz), rng)

			spawned_buildings.append({
				"position": Vector3(bx, altitude, bz),
				"type": building_type,
				"cluster": i
			})

func spawn_wildlife():
	spawned_animals.clear()

	var herd_positions = []
	var herd_count = rng.randi_range(4, 10)
	for i in range(herd_count):
		var hx = rng.randf_range(world_bounds.min_x * 0.5, world_bounds.max_x * 0.5)
		var hz = rng.randf_range(world_bounds.min_z * 0.5, world_bounds.max_z * 0.5)
		herd_positions.append(Vector2(hx, hz))

	for herd_pos in herd_positions:
		var herd_size = rng.randi_range(3, 8)
		var animal_type = rng.randi() % 5

		for i in range(herd_size):
			var ax = herd_pos.x + rng.randf_range(-20, 20)
			var az = herd_pos.y + rng.randf_range(-20, 20)
			var altitude = terrain.get_height_at(ax, az) if terrain else rng.randf_range(0, 10)

			spawned_animals.append({
				"position": Vector3(ax, altitude, az),
				"type": animal_type,
				"herd": herd_pos
			})

			animal_spawner.spawn_animal(animal_type, Vector3(ax, altitude, az), rng)

func get_environment_stats() -> Dictionary:
	var fps = Engine.get_frames_per_second()
	var mem = OS.get_static_memory_usage() / 1048576.0
	var nodes = Performance.get_monitor(Performance.OBJECT_NODE_COUNT)
	var draw_calls = Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	var objects = Performance.get_monitor(Performance.OBJECT_COUNT)
	var physics = Performance.get_monitor(Performance.PHYSICS_3D_ACTIVE_OBJECTS)
	var physics_time = Performance.get_monitor(Performance.PHYSICS_3D_ISLAND_COUNT)

	return {
		"fps": fps,
		"memory_mb": mem,
		"nodes": nodes,
		"objects": objects,
		"draw_calls": draw_calls,
		"physics_objects": physics,
		"physics_islands": physics_time,
		"trees": spawned_trees.size(),
		"animals": spawned_animals.size(),
		"buildings": spawned_buildings.size(),
		"world_seed": world_seed,
	}

class AnimalSpawner:
	extends Node

	func spawn_animal(type: int, position: Vector3, rng: RandomNumberGenerator):
		pass

func _get_biome_at(x: float, z: float) -> String:
	var altitude = terrain.get_height_at(x, z) if terrain else 0
	for biome_name in biomes:
		var cfg = biomes[biome_name]
		if altitude >= cfg.altitude_min and altitude <= cfg.altitude_max:
			return biome_name
	return "sparse_woods"