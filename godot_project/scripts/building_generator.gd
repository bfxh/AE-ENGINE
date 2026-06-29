extends Node3D
class_name BuildingGenerator

var building_parent: Node3D
var building_count: int = 35
var spawn_radius: float = 220.0
var spawn_seed: int = 1337

func _ready():
	building_parent = Node3D.new()
	building_parent.name = "Buildings"
	add_child(building_parent)
	generate_buildings()

func generate_buildings():
	var rng = RandomNumberGenerator.new()
	rng.seed = spawn_seed

	var building_mats = _create_building_materials()

	for i in range(building_count):
		var angle = rng.randf() * TAU
		var dist = rng.randf_range(30.0, spawn_radius)
		var x = cos(angle) * dist
		var z = sin(angle) * dist

		var building_type = rng.randi() % 7
		var building = _generate_building(building_type, rng)
		building.position = Vector3(x, 0, z)
		building.rotation_degrees.y = rng.randf() * 360.0

		var mat_idx = rng.randi() % building_mats.size()
		building.material_override = building_mats[mat_idx]

		building_parent.add_child(building)

func _create_building_materials() -> Array:
	var mats = []

	var concrete_mat = StandardMaterial3D.new()
	concrete_mat.albedo_color = Color(0.5, 0.48, 0.45)
	concrete_mat.roughness = 0.85
	concrete_mat.metallic = 0.05
	mats.append(concrete_mat)

	var rusted_metal = StandardMaterial3D.new()
	rusted_metal.albedo_color = Color(0.45, 0.25, 0.1)
	rusted_metal.roughness = 0.7
	rusted_metal.metallic = 0.3
	mats.append(rusted_metal)

	var brick = StandardMaterial3D.new()
	brick.albedo_color = Color(0.4, 0.2, 0.12)
	brick.roughness = 0.9
	brick.metallic = 0.0
	mats.append(brick)

	return mats

func _generate_building(type: int, rng: RandomNumberGenerator) -> MeshInstance3D:
	var mesh_instance = MeshInstance3D.new()
	match type:
		0:
			mesh_instance.mesh = _make_tower(rng, false)
			mesh_instance.create_trimesh_collision()
		1:
			mesh_instance.mesh = _make_tower(rng, true)
			mesh_instance.create_trimesh_collision()
		2:
			mesh_instance.mesh = _make_house(rng, false)
			mesh_instance.create_trimesh_collision()
		3:
			mesh_instance.mesh = _make_house(rng, true)
			mesh_instance.create_trimesh_collision()
		4:
			mesh_instance.mesh = _make_warehouse(rng)
			mesh_instance.create_trimesh_collision()
		5:
			mesh_instance.mesh = _make_ruin(rng)
			mesh_instance.create_trimesh_collision()
		_:
			mesh_instance.mesh = _make_ruin(rng)
			mesh_instance.create_trimesh_collision()

	return mesh_instance

func _make_tower(rng: RandomNumberGenerator, damaged: bool) -> BoxMesh:
	var mesh = BoxMesh.new()
	var w = rng.randf_range(3.0, 6.0)
	var d = rng.randf_range(3.0, 6.0)
	var h = rng.randf_range(15.0, 40.0)
	mesh.size = Vector3(w, h, d)
	return mesh

func _make_house(rng: RandomNumberGenerator, _damaged: bool) -> ArrayMesh:
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	var w = rng.randf_range(6.0, 12.0)
	var d = rng.randf_range(5.0, 10.0)
	var h = rng.randf_range(4.0, 8.0)

	var v000 = Vector3(-w/2, 0, -d/2)
	var v100 = Vector3(w/2, 0, -d/2)
	var v010 = Vector3(-w/2, 0, d/2)
	var v110 = Vector3(w/2, 0, d/2)
	var v001 = Vector3(-w/2, h, -d/2)
	var v101 = Vector3(w/2, h, -d/2)
	var v011 = Vector3(-w/2, h, d/2)
	var v111 = Vector3(w/2, h, d/2)

	_add_quad(surface_tool, v000, v100, v101, v001)
	_add_quad(surface_tool, v100, v110, v111, v101)
	_add_quad(surface_tool, v110, v010, v011, v111)
	_add_quad(surface_tool, v010, v000, v001, v011)

	var roof_h = h + rng.randf_range(2.0, 4.0)
	var ridge = Vector3(0, roof_h, 0)
	_add_tri(surface_tool, v001, v101, ridge)
	_add_tri(surface_tool, v101, v111, ridge)
	_add_tri(surface_tool, v111, v011, ridge)
	_add_tri(surface_tool, v011, v001, ridge)

	surface_tool.generate_normals()

	var arr_mesh = surface_tool.commit()
	return arr_mesh

func _make_warehouse(rng: RandomNumberGenerator) -> BoxMesh:
	var mesh = BoxMesh.new()
	var w = rng.randf_range(10.0, 20.0)
	var d = rng.randf_range(8.0, 15.0)
	var h = rng.randf_range(5.0, 10.0)
	mesh.size = Vector3(w, h, d)
	return mesh

func _make_ruin(rng: RandomNumberGenerator) -> ArrayMesh:
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	var w = rng.randf_range(4.0, 10.0)
	var d = rng.randf_range(4.0, 8.0)
	var h = rng.randf_range(2.0, 6.0)

	var v000 = Vector3(-w/2, 0, -d/2)
	var v100 = Vector3(w/2, 0, -d/2)
	var v010 = Vector3(-w/2, 0, d/2)
	var v110 = Vector3(w/2, 0, d/2)
	var v001 = Vector3(-w/2, h, -d/2)
	var v101 = Vector3(w/2, h, -d/2)
	var v011 = Vector3(-w/2, h, d/2)
	var v111 = Vector3(w/2, h, d/2)

	_add_quad(surface_tool, v000, v100, v101, v001)
	_add_quad(surface_tool, v100, v110, v111, v101)
	_add_quad(surface_tool, v110, v010, v011, v111)

	surface_tool.generate_normals()

	var arr_mesh = surface_tool.commit()
	return arr_mesh

func _add_quad(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3):
	st.add_vertex(a); st.add_vertex(b); st.add_vertex(c)
	st.add_vertex(a); st.add_vertex(c); st.add_vertex(d)

func _add_tri(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3):
	st.add_vertex(a); st.add_vertex(b); st.add_vertex(c)