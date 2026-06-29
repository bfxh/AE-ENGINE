extends Node3D
class_name WaterBodySystem

var water_parent: Node3D
var lake_count: int = 3
var pond_count: int = 8
var stream_count: int = 4
var spawn_seed: int = 555
var world_radius: float = 230.0

func _ready():
	water_parent = Node3D.new()
	water_parent.name = "WaterBodies"
	add_child(water_parent)
	generate_all()

func generate_all():
	var rng = RandomNumberGenerator.new()
	rng.seed = spawn_seed

	generate_lakes(rng)
	generate_ponds(rng)
	generate_streams(rng)

func generate_lakes(rng: RandomNumberGenerator):
	var water_mat = _create_water_material(Color(0.15, 0.35, 0.6), 0.3)

	for i in range(lake_count):
		var angle = float(i) / float(lake_count) * TAU + rng.randf_range(-0.2, 0.2)
		var dist = rng.randf_range(50.0, world_radius * 0.6)
		var cx = cos(angle) * dist
		var cz = sin(angle) * dist

		var base_h = _get_ground_height(cx, cz) - 1.0
		if base_h < -30:
			base_h = rng.randf_range(-5, 3)

		var lake = Node3D.new()
		lake.name = "Lake_%d" % i

		var segments = rng.randi_range(8, 16)
		var lake_radius = rng.randf_range(15.0, 40.0)

		var st = SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLES)

		var center = Vector3(cx, base_h + 0.2, cz)
		var edge_particles = []
		for s in range(segments):
			var sa = float(s) / float(segments) * TAU
			var r_var = lake_radius * rng.randf_range(0.6, 1.2)
			var ex = cx + cos(sa) * r_var
			var ez = cz + sin(sa) * r_var
			edge_particles.append(Vector3(ex, base_h + 0.2, ez))

		for s in range(segments):
			var next = (s + 1) % segments
			st.add_vertex(center)
			st.add_vertex(edge_particles[s])
			st.add_vertex(edge_particles[next])

		st.generate_normals()
		var lake_mesh = MeshInstance3D.new()
		lake_mesh.mesh = st.commit()
		lake_mesh.material_override = water_mat

		var depth_vis = MeshInstance3D.new()
		depth_vis.mesh = st.commit()
		depth_vis.position.y = -rng.randf_range(1.0, 4.0)
		var deep_mat = water_mat.duplicate()
		deep_mat.albedo_color = deep_mat.albedo_color.darkened(0.3)
		depth_vis.material_override = deep_mat

		lake.add_child(depth_vis)
		lake.add_child(lake_mesh)
		water_parent.add_child(lake)

func generate_ponds(rng: RandomNumberGenerator):
	var water_mat = _create_water_material(Color(0.2, 0.4, 0.5), 0.4)

	for i in range(pond_count):
		var px = rng.randf_range(-world_radius, world_radius)
		var pz = rng.randf_range(-world_radius, world_radius)

		var h = _get_ground_height(px, pz) - 0.3
		if h < -30:
			continue

		var pond_radius = rng.randf_range(2.0, 8.0)
		var pond_mesh = MeshInstance3D.new()
		var c_mesh = CylinderMesh.new()
		c_mesh.top_radius = pond_radius
		c_mesh.bottom_radius = pond_radius * 0.9
		c_mesh.height = 0.1
		pond_mesh.mesh = c_mesh
		pond_mesh.material_override = water_mat
		pond_mesh.position = Vector3(px, h + 0.2, pz)
		pond_mesh.scale = Vector3(1.0, 1.0, rng.randf_range(0.5, 1.0))

		water_parent.add_child(pond_mesh)

func generate_streams(rng: RandomNumberGenerator):
	var water_mat = _create_water_material(Color(0.2, 0.45, 0.7), 0.35)

	for i in range(stream_count):
		var sx = rng.randf_range(-world_radius * 0.7, world_radius * 0.7)
		var sz = rng.randf_range(-world_radius * 0.7, world_radius * 0.7)

		var start_h = _get_ground_height(sx, sz)
		if start_h < -20:
			continue

		var points = PackedVector3Array()
		var px = sx
		var pz = sz
		var length = rng.randf_range(30.0, 80.0)
		var direction = rng.randf() * TAU
		var step_count = int(length / 3.0)

		for s in range(step_count):
			px += cos(direction) * 3.0 + rng.randf_range(-1.5, 1.5)
			pz += sin(direction) * 3.0 + rng.randf_range(-1.5, 1.5)
			var py = _get_ground_height(px, pz)
			if py < -20:
				py = points[points.size() - 1].y if points.size() > 0 else start_h
			py = min(py, (points[points.size() - 1].y if points.size() > 0 else start_h))
			points.append(Vector3(px, py + 0.1, pz))

		if points.size() < 3:
			continue

		var st = SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLE_STRIP)

		var stream_width = rng.randf_range(1.0, 4.0)
		for j in range(points.size() - 1):
			var a = points[j]
			var b = points[j + 1]
			var dir = (b - a).normalized()
			var perp = Vector3(-dir.z, 0, dir.x) * stream_width * 0.5

			st.add_vertex(a + perp)
			st.add_vertex(a - perp)
			st.add_vertex(b + perp)
			st.add_vertex(b - perp)

		st.generate_normals()
		var stream_mesh = MeshInstance3D.new()
		stream_mesh.mesh = st.commit()
		stream_mesh.material_override = water_mat
		water_parent.add_child(stream_mesh)

func _create_water_material(color: Color, metallic: float) -> StandardMaterial3D:
	var mat = StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = 0.2
	mat.metallic = metallic
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color.a = 0.75
	mat.refraction_enabled = true
	mat.refraction_scale = 0.05
	return mat

func _get_ground_height(x: float, z: float) -> float:
	var space_state = get_world_3d().direct_space_state
	if space_state:
		var ray_query = PhysicsRayQueryParameters3D.create(Vector3(x, 500, z), Vector3(x, -50, z))
		var result = space_state.intersect_ray(ray_query)
		if result:
			return result.position.y
	return -999.0