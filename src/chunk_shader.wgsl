
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};


@vertex
fn vs_main(vertex: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(vertex.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.8, 0.8, 0.8, 0.6);  // Default chunk color
}