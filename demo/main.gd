extends Node3D

var world: WastelandWorld

func _ready():
	print("[Demo] Wasteland Engine Demo initialized")
	world = $WastelandWorld
	if world:
		world.initialize(100.0, 100.0, 100.0)
		print("[Demo] World initialized: ", world.get_stats())
		
		# Spawn some entities
		for i in range(50):
			var x = randf_range(-20.0, 20.0)
			var y = randf_range(-5.0, 5.0)
			var z = randf_range(-20.0, 20.0)
			world.add_voxel_grid(x, y, z, 2.0)
		
		# Spawn NPCs
		for i in range(10):
			var x = randf_range(-15.0, 15.0)
			var z = randf_range(-15.0, 15.0)
			world.add_npc("NPC_" + str(i), x, 0.0, z, "human", "neutral")
	else:
		print("[Demo] ERROR: WastelandWorld node not found!")

func _process(delta):
	if world:
		world.time_scale = 1.0
		var stats = world.get_stats()
		if Engine.get_process_frames() % 60 == 0:
			print("[Demo] tick: ", stats)