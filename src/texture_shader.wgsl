// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>, // New: Pass color to fragment shader
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;

    // Convert integer position to float (assumed fixed from previous code)
    let pos_f32 = vec3<f32>(model.position);

    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(pos_f32, 1.0);
    out.uv = model.uv;

    // New: Generate color from normals (for debugging face orientation)
    let normalized_color = (model.normal + vec3<f32>(1.0)) * 0.5; // [-1,1] → [0,1]
    out.color = vec4<f32>(normalized_color, 1.0);

    return out;
}

// Fragment shader (texture sampling + color tint)
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(t_diffuse, s_diffuse, in.uv);
    return texture_color * in.color; // Multiply texture by vertex color
}