//! Shader 库：内嵌 WGSL shader + 缓存

use wgpu::{Device, ShaderModule};

/// 内嵌的 PBR 主 shader
pub const PBR_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

struct MaterialParams {
    base_color: vec4<f32>,
    metallic_roughness: vec4<f32>,
    emissive: vec4<f32>,
    flags: u32,
    normal_scale: f32,
    occlusion_strength: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;
@group(2) @binding(0) var base_color_texture: texture_2d<f32>;
@group(2) @binding(1) var normal_texture: texture_2d<f32>;
@group(2) @binding(2) var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(3) var occlusion_texture: texture_2d<f32>;
@group(2) @binding(4) var emissive_texture: texture_2d<f32>;
@group(2) @binding(5) var material_sampler: sampler;
@group(2) @binding(6) var<uniform> material: MaterialParams;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) view_dir: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = model.model * vec4<f32>(input.position, 1.0);
    output.world_position = world_pos.xyz;
    output.world_normal = normalize((model.normal_matrix * vec4<f32>(input.normal, 0.0)).xyz);
    output.world_tangent = vec4<f32>(normalize((model.normal_matrix * vec4<f32>(input.tangent.xyz, 0.0)).xyz), input.tangent.w);
    output.uv = input.uv;
    output.color = input.color;
    output.clip_position = camera.view_proj * world_pos;
    output.view_dir = normalize(camera.position.xyz - world_pos.xyz);
    return output;
}

fn has_flag(flag: u32) -> bool {
    return (material.flags & flag) != 0u;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let FLAG_BASE_COLOR_TEX = 1u;
    let FLAG_NORMAL_TEX = 2u;
    let FLAG_MR_TEX = 4u;
    let FLAG_OCCLUSION_TEX = 8u;
    let FLAG_EMISSIVE_TEX = 16u;
    let FLAG_UNLIT = 256u;

    var base_color = material.base_color;
    if (has_flag(FLAG_BASE_COLOR_TEX)) {
        base_color = base_color * textureSample(base_color_texture, material_sampler, input.uv);
    }
    base_color = base_color * input.color;

    if (has_flag(FLAG_UNLIT)) {
        return vec4<f32>(base_color.rgb, base_color.a);
    }

    var metallic = material.metallic_roughness.x;
    var roughness = material.metallic_roughness.y;
    if (has_flag(FLAG_MR_TEX)) {
        let mr = textureSample(metallic_roughness_texture, material_sampler, input.uv);
        metallic = mr.b * metallic;
        roughness = mr.g * roughness;
    }

    var normal = input.world_normal;
    if (has_flag(FLAG_NORMAL_TEX)) {
        let tangent_normal = textureSample(normal_texture, material_sampler, input.uv).xyz * 2.0 - 1.0;
        let t = normalize(input.world_tangent.xyz);
        let b = cross(normal, t) * input.world_tangent.w;
        let tbn = mat3x3<f32>(t, b, normal);
        normal = normalize(tbn * tangent_normal * material.normal_scale);
    }

    // 简化 PBR：环境光 + 主方向光
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let light_color = vec3<f32>(1.0, 0.95, 0.9) * 2.5;
    let ambient = vec3<f32>(0.15, 0.18, 0.22);

    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let half_dir = normalize(light_dir + input.view_dir);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let n_dot_v = max(dot(normal, input.view_dir), 0.0);

    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;

    let d = alpha2 / (3.14159 * pow(n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0, 2.0));
    let g = n_dot_l / (n_dot_l * (1.0 - k) + k) * n_dot_v / (n_dot_v * (1.0 - k) + k);
    let f0 = mix(vec3<f32>(0.04), base_color.rgb, metallic);
    let f = f0 + (1.0 - f0) * pow(1.0 - n_dot_v, 5.0);

    let specular = d * g * f / (4.0 * n_dot_l * n_dot_v + 0.001);
    let kd = (1.0 - metallic) * (1.0 - f);

    let diffuse = kd * base_color.rgb / 3.14159;

    var ao = 1.0;
    if (has_flag(FLAG_OCCLUSION_TEX)) {
        ao = mix(1.0, textureSample(occlusion_texture, material_sampler, input.uv).r, material.occlusion_strength);
    }

    var emissive = material.emissive.rgb * material.metallic_roughness.z;
    if (has_flag(FLAG_EMISSIVE_TEX)) {
        emissive = emissive + textureSample(emissive_texture, material_sampler, input.uv).rgb;
    }

    let color = (ambient + diffuse * n_dot_l * light_color + specular * n_dot_l * light_color) * ao + emissive;
    return vec4<f32>(color, base_color.a);
}
"#;

/// 内嵌的网格线 shader
pub const GRID_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

/// 内嵌的深度 pre-pass shader
pub const DEPTH_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_proj * model.model * vec4<f32>(input.position, 1.0);
}
"#;

/// Shader 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderId {
    Pbr,
    Grid,
    Depth,
}

impl ShaderId {
    pub fn source(self) -> &'static str {
        match self {
            Self::Pbr => PBR_SHADER,
            Self::Grid => GRID_SHADER,
            Self::Depth => DEPTH_SHADER,
        }
    }
}

/// Shader 库：缓存已编译的 ShaderModule
pub struct ShaderLibrary {
    pbr: Option<ShaderModule>,
    grid: Option<ShaderModule>,
    depth: Option<ShaderModule>,
}

impl Default for ShaderLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderLibrary {
    pub fn new() -> Self {
        Self { pbr: None, grid: None, depth: None }
    }

    pub fn get(&mut self, device: &Device, id: ShaderId) -> &ShaderModule {
        match id {
            ShaderId::Pbr => self.pbr.get_or_insert_with(|| {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("pbr shader"),
                    source: wgpu::ShaderSource::Wgsl(PBR_SHADER.into()),
                })
            }),
            ShaderId::Grid => self.grid.get_or_insert_with(|| {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("grid shader"),
                    source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
                })
            }),
            ShaderId::Depth => self.depth.get_or_insert_with(|| {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("depth shader"),
                    source: wgpu::ShaderSource::Wgsl(DEPTH_SHADER.into()),
                })
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_sources_nonempty() {
        assert!(!PBR_SHADER.is_empty());
        assert!(!GRID_SHADER.is_empty());
        assert!(!DEPTH_SHADER.is_empty());
    }

    #[test]
    fn shader_id_source_mapping() {
        assert_eq!(ShaderId::Pbr.source(), PBR_SHADER);
        assert_eq!(ShaderId::Grid.source(), GRID_SHADER);
        assert_eq!(ShaderId::Depth.source(), DEPTH_SHADER);
    }

    #[test]
    fn pbr_shader_has_vertex_and_fragment() {
        assert!(PBR_SHADER.contains("@vertex"));
        assert!(PBR_SHADER.contains("@fragment"));
        assert!(PBR_SHADER.contains("vs_main"));
        assert!(PBR_SHADER.contains("fs_main"));
    }

    #[test]
    fn shader_library_default() {
        let lib = ShaderLibrary::default();
        assert!(lib.pbr.is_none());
        assert!(lib.grid.is_none());
        assert!(lib.depth.is_none());
    }
}
