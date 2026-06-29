struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    selected: u32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_normal = normalize((model.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.world_pos = world_pos.xyz;
    out.color = in.color * model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;

    var color = in.color * lighting;

    if model.selected > 0u {
        let edge = 1.0 - smoothstep(0.0, 0.05, abs(dot(in.world_normal, normalize(camera.camera_pos - in.world_pos))));
        color = mix(color, vec4<f32>(1.0, 0.6, 0.0, 1.0), edge * 0.5);
    }

    return color;
}