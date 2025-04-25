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

@vertex
fn vs_main(vertex: VertexInput) -> @builtin(position) vec4f {
    return camera.view_proj * vec4f(vertex.position + chunk.position, 1f);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(0.3, 0.3, 0.3, 0.4);  // Default chunk color
}