struct CameraUniform {
    view_proj: mat4x4f,
};
struct ChunkUniform {
    position: vec3f,
};

@group(1) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var<uniform> chunk: ChunkUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    // First apply chunk position (as translation), then camera view_proj
    let world_position = vec4f(vertex.position + chunk.position, 1f);
    return VertexOutput(
        camera.view_proj * world_position,
        vertex.uv
    );
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}