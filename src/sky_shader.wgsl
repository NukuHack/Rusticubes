
struct VertexOutput {
    @builtin(position) clip_position: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2f, 3>(
        vec2f(-1.0, -1.0),
        vec2f(3.0, -1.0),
        vec2f(-1.0, 3.0),
    );
    
    var out: VertexOutput;
    out.clip_position = vec4f(positions[vertex_index], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4f {
    // Sky color (light blue)
    return vec4f(0.1, 0.2, 0.3, 1.0);
}