struct CameraUniform {
	view_proj: mat4x4f,
};
struct ChunkUniform {
	position: u64,
};

@group(1) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var<uniform> chunk: ChunkUniform;

// Constants matching your Rust implementation
const Z_SHIFT: u32 = 0u;
const Y_SHIFT: u32 = 26u;
const X_SHIFT: u32 = 38u;

const Z_MASK: u64 = u64(0x3FFFFFFu); // (1 << 26) - 1
const Y_MASK: u64 = u64(0xFFFu);     // (1 << 12) - 1
const X_MASK: u64 = u64(0x3FFFFFFu); // (1 << 26) - 1

// Assuming CHUNK_SIZE_I is passed as a uniform or constant
const CHUNK_SIZE_I: i32 = 16i; // Adjust based on your actual chunk size

// Helper functions to extract coordinates from the packed u64
fn extract_x(coord: u64) -> i32 {
	let x = i32((coord >> X_SHIFT) & X_MASK);
	// Sign extension for 26-bit value
	return (x << 6u) >> 6u;
}

fn extract_y(coord: u64) -> i32 {
	let y = i32((coord >> Y_SHIFT) & Y_MASK);
	// Sign extension for 12-bit value
	return (y << 20u) >> 20u;
}

fn extract_z(coord: u64) -> i32 {
	let z = i32(coord & Z_MASK);
	// Sign extension for 26-bit value
	return (z << 6u) >> 6u;
}

// Main function to convert ChunkCoord to world position
fn to_world_pos(coord: u64) -> vec3<f32> {
	let x = extract_x(coord);
	let y = extract_y(coord);
	let z = extract_z(coord);
	
	return vec3<f32>(
		f32(x * CHUNK_SIZE_I),
		f32(y * CHUNK_SIZE_I),
		f32(z * CHUNK_SIZE_I)
	);
}

struct VertexInput {
	@location(0) packed_data: u32,
};
/*
// normal's ordering
[0] Left face
[1] Right face
[2] Front face
[3] Back face
[4] Top face
[5] Bottom face
*/

struct VertexOutput {
	@builtin(position) clip_position: vec4f,
	@location(0) uv: vec2f,
};


@vertex
fn vs_main(
	vertex: VertexInput,
	@builtin(vertex_index) vert_idx: u32  // Built-in vertex index
) -> VertexOutput {
	// Unpack x, y, z (each 5 bits, stored in bits 0-14 of the u32)
	let x = f32((vertex.packed_data >> 0u)  & 0x1Fu);  // bits 0-4  (mask = 0b11111 or 0x1F)
	let y = f32((vertex.packed_data >> 5u)  & 0x1Fu);  // bits 5-9  (shift by 5, mask 0x1F)
	let z = f32((vertex.packed_data >> 10u) & 0x1Fu);  // bits 10-14 (shift by 10, mask 0x1F)
	// Note: Uses 15 bits total (5 bits per axis)

	let normal = vertex.packed_data >> 16u;


	var out: VertexOutput;
	// First apply chunk position (as translation), then camera view_proj
	let world_pos = vec3<f32>(x, y, z) + to_world_pos(chunk.position);
	out.clip_position = camera.view_proj * vec4f(world_pos, 1f);
	
	// Calculate UV based on vertex index within the current quad
	// Since each quad has 4 vertices, we use modulo 4 to get the local vertex index
	var local_vertex = vert_idx % 4u;
	if local_vertex == 2u { // 2 and 3 needs to be switched because I'm creating them in a strange order
		local_vertex = 3u;
	} else if local_vertex == 3u {
		local_vertex = 2u;
	}
	// Custom quad UV mapping:
	// Vertex 0: (0, 0) - bottom-left
	// Vertex 1: (1, 0) - bottom-right
	// Vertex 2: (0, 1) - top-left
	// Vertex 3: (1, 1) - top-right
	out.uv = vec2<f32>(f32(local_vertex & 1u), f32(local_vertex >> 1u));
		
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
