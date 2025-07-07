


@vertex
fn vs_main(
    vertex: VertexInput,
    @builtin(vertex_index) vert_idx: u32  // Built-in vertex index
) -> VertexOutput {
    var out: VertexOutput;
    // First apply chunk position (as translation), then camera view_proj
    out.clip_position = camera.view_proj * vec4f(vertex.position + to_world_pos(chunk.position), 1f);
    out.uv = vertex.uv;

    /*
    // Calculate UVs based on vertex index (for a quad made of two triangles)
    out.uv = vec2<f32>(
        select(0.0, 1.0, (vert_idx & 1u) == 1u),  // U (horizontal) coordinate
        select(1.0, 0.0, (vert_idx & 2u) == 2u)   // V (vertical) coordinate
    );
    */
    return out;
}


// The rest of your shader can remain the same
@group(0) @binding(0) var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
// tbh only one more bind group can be used, after that it might not be supported on all devices
/*
struct Uniforms {
    data: u32,
};
@group(3) @binding(0) var<uniform> data: Uniforms;
*/

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
/*
    // For solid color rectangles
    if (in.uv.x == 0.0 && in.uv.y == 0.0) {
        return vec4<f32>(0.0,0.0,0.0,0.0);
    }
*/
    return textureSample(t_diffuse, s_diffuse, in.uv, 0);
}
