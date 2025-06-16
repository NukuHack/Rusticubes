


@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    // First apply chunk position (as translation), then camera view_proj
    let world_position = camera.view_proj * vec4f(vertex.position + to_world_pos(chunk.position), 1f);
    return VertexOutput(
        world_position,
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