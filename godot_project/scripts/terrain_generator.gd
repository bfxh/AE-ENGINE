extends Node3D
class_name TerrainGenerator

var terrain_size: float = 500.0
var terrain_resolution: int = 256
var height_scale: float = 30.0
var noise_seed: int = 42
var terrain_mesh: MeshInstance3D

func _ready():
	generate_terrain()

func generate_terrain():
	terrain_mesh = MeshInstance3D.new()
	terrain_mesh.name = "Terrain"
	add_child(terrain_mesh)

	var noise = FastNoiseLite.new()
	noise.seed = noise_seed
	noise.frequency = 0.005
	noise.fractal_octaves = 5
	noise.fractal_lacunarity = 2.0
	noise.fractal_gain = 0.5

	var noise2 = FastNoiseLite.new()
	noise2.seed = noise_seed + 100
	noise2.frequency = 0.02
	noise2.fractal_octaves = 3

	var noise3 = FastNoiseLite.new()
	noise3.seed = noise_seed + 200
	noise3.frequency = 0.001
	noise3.fractal_octaves = 2

	var plane = PlaneMesh.new()
	plane.size = Vector2(terrain_size, terrain_size)
	plane.subdivide_depth = terrain_resolution / 2
	plane.subdivide_width = terrain_resolution / 2

	var surface_tool = SurfaceTool.new()
	surface_tool.create_from(plane, 0)

	var mesh_data = surface_tool.commit()
	var arrays = mesh_data.surface_get_arrays(0)
	var vertices = arrays[Mesh.ARRAY_VERTEX]

	var new_vertices = PackedVector3Array()
	var new_colors = PackedColorArray()

	for v in vertices:
		var x = v.x
		var z = v.z

		var h1 = noise.get_noise_2d(x, z) * height_scale
		var h2 = noise2.get_noise_2d(x, z) * height_scale * 0.3
		var h3 = noise3.get_noise_2d(x, z) * height_scale * 0.5

		var h = h1 + h2 + h3
		h += abs(noise.get_noise_2d(x * 0.3, z * 0.3)) * height_scale * 0.2

		h = max(h, -height_scale * 0.3)
		h = min(h, height_scale * 1.2)

		new_vertices.append(Vector3(x, h, z))

		var t = (h + height_scale * 0.3) / (height_scale * 1.5)
		t = clamp(t, 0.0, 1.0)

		var color = _get_terrain_color(t, h, noise.get_noise_2d(x * 0.1, z * 0.1))
		new_colors.append(color)

	arrays[Mesh.ARRAY_VERTEX] = new_vertices
	arrays[Mesh.ARRAY_COLOR] = new_colors

	var arr_mesh = ArrayMesh.new()
	arr_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)

	var mat = StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.9
	mat.metallic = 0.0

	terrain_mesh.mesh = arr_mesh
	terrain_mesh.material_override = mat
	terrain_mesh.create_trimesh_collision()

func _get_terrain_color(t: float, _h: float, detail: float) -> Color:
	if t < 0.15:
		return Color(0.15, 0.12, 0.08)
	elif t < 0.3:
		return Color(0.35 + detail * 0.1, 0.25 + detail * 0.05, 0.15 + detail * 0.05)
	elif t < 0.5:
		return Color(0.4 + detail * 0.05, 0.3 + detail * 0.05, 0.2 + detail * 0.05)
	elif t < 0.7:
		return Color(0.35, 0.38, 0.3)
	elif t < 0.9:
		return Color(0.45, 0.45, 0.4)
	else:
		return Color(0.5, 0.48, 0.45)