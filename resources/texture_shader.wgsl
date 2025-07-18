
@vertex
fn vs_main(
	vertex: VertexInput,
	@builtin(vertex_index) vert_idx: u32  // Built-in vertex index
) -> VertexOutput {
	var out: VertexOutput;
	// First apply chunk position (as translation), then camera view_proj
	out.clip_position = camera.view_proj * vec4f(to_bytes(vertex.packed_data) + to_world_pos(chunk.position), 1f);
	
	// Calculate UV based on vertex index within the current quad
	// Since each quad has 4 vertices, we use modulo 4 to get the local vertex index
	var local_vertex = vert_idx % 4u;
	
	// Standard quad UV mapping:
	// Vertex 0: (0, 0) - bottom-left
	// Vertex 1: (1, 0) - bottom-right  
	// Vertex 2: (1, 1) - top-right
	// Vertex 3: (0, 1) - top-left
	var quad_uv = vec2<f32>(f32(local_vertex & 1u), f32(local_vertex >> 1u));
	
	// Since we removed UV data from vertices, we can either:
	// 1. Use the quad_uv directly for a simple repeating texture
	// 2. Apply some procedural mapping based on world position
	// 3. Use a uniform texture atlas coordinate if you have one
	
	// Option 1: Simple repeating texture across all faces
	out.uv = quad_uv;
	
	// Option 2: Scale the UVs if you want tiling
	// out.uv = quad_uv * 2.0; // This would tile the texture 2x2 on each face
	
	// Option 3: Use world position for procedural texturing
	// var world_pos = to_bytes(vertex.packed_data) + to_world_pos(chunk.position);
	// out.uv = vec2<f32>(world_pos.x * 0.1, world_pos.z * 0.1); // Scale as needed
	
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
