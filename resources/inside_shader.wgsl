@vertex
fn vs_main(vertex: VertexInput) -> @builtin(position) vec4f {
    return camera.view_proj * vec4f(to_bytes(vertex.packed_data) +  to_world_pos(chunk.position), 1f);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(0.3, 0.3, 0.3, 0.4);  // Default chunk color
}