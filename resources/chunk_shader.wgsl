
@group(1) @binding(0) var<uniform> camera_proj: mat4x4f;
@group(2) @binding(0) var<uniform> chunk_pos: u64;

// Assuming CHUNK_SIZE_I is passed as a uniform or constant
const CHUNK_SIZE_I: i32 = 16i; // Adjust based on your actual chunk size

fn extract_x(coord: u64) -> i32 {
	let x = i32((coord >> 38u) & u64(0x3FFFFFFu));
	// Simplified sign extension using arithmetic shift
	return (x << 38u) >> 38u;  // Matches 26-bit sign extension
}
fn extract_y(coord: u64) -> i32 {
	let y = i32((coord >> 26u) & u64(0xFFFu));
	// Simplified sign extension
	return (y << 52u) >> 52u;  // Matches 12-bit sign extension
}
fn extract_z(coord: u64) -> i32 {
	let z = i32(coord & u64(0x3FFFFFFu));
	// Simplified sign extension
	return (z << 38u) >> 38u;  // Matches 26-bit sign extension
}
// Main function to convert ChunkCoord to world position
fn to_world_pos(coord: u64) -> vec3f {
	let x = extract_x(coord);
	let y = extract_y(coord);
	let z = extract_z(coord);
	
	return vec3f(
		f32(x * CHUNK_SIZE_I),
		f32(y * CHUNK_SIZE_I),
		f32(z * CHUNK_SIZE_I)
	);
}

fn normal_to_rot(pos: vec3f, normal: u32) -> vec3f {
	var out = pos;
	if normal == 0 {
		out = vec3f(pos.y, pos.x, pos.z);  // Left face
	} else if normal == 1 {
		out = vec3f(pos.y+1.0, (-pos.x)+1.0, pos.z);// Right face
	} else if normal == 2 {
		out = vec3f(pos.x, pos.z, pos.y);   // Front face
	} else if normal == 3 {
		out = vec3f(pos.x, (-pos.z)+1.0, pos.y+1.0);// Back face
	} else if normal == 4 {
		out = vec3f(pos.x, pos.y + 1.0, pos.z);  // Top face
	} else if normal == 5 {
		out = vec3f(pos.x, -pos.y, (-pos.z)+1.0); // Bottom face
	}
	return out;
}

const NORMALS: array<vec3f, 6> = array<vec3f, 6>(
	vec3f(-1.0, 0.0, 0.0),   // [0] Left face
	vec3f(1.0, 0.0, 0.0),    // [1] Right face
	vec3f(0.0, 0.0, -1.0),   // [2] Front face
	vec3f(0.0, 0.0, 1.0),    // [3] Back face
	vec3f(0.0, 1.0, 0.0),    // [4] Top face
	vec3f(0.0, -1.0, 0.0)    // [5] Bottom face
);

struct VertexOutput {
	@builtin(position) clip_position: vec4f,
	@location(0) world_normal: vec3f,
	@location(1) uv: vec2f,
};

@vertex
fn vs_main(
	@location(0) vertex_data: u32,
	@location(1) instance_data: u32,
	@builtin(vertex_index) vert_idx: u32
) -> VertexOutput {
	// Unpack vertex position (5 bits per axis)
	let vertex_pos = vec3f(
		f32((vertex_data >> 0u) & 0x1Fu),
		f32((vertex_data >> 5u) & 0x1Fu),
		f32((vertex_data >> 10u) & 0x1Fu)
		);
	
	// Unpack instance position (4 bits per axis) 0-15 as position
	let instance_pos = vec3f(
		f32((instance_data >> 0u) & 0xFu),
		f32((instance_data >> 4u) & 0xFu),
		f32((instance_data >> 8u) & 0xFu)
		);
	
	// Get normal from instance data (bits 16-18)
	let normal_idx = (instance_data >> 12u) & 0x7u;

	let model_pos = normal_to_rot(vertex_pos, normal_idx); // Combine vertex and instance positions
	
	let normal = NORMALS[normal_idx];
	
	var output: VertexOutput;
	
	// Apply chunk position (as translation), then camera view_proj
	let world_pos = to_world_pos(chunk_pos) + model_pos + instance_pos;
	output.clip_position = camera_proj * vec4f(world_pos, 1.0);
	
	output.world_normal = normal;
	
	// Calculate UV based on original vertex positions
	// Since your quad is defined with positions:
	// [0,0,0], [0,0,1], [1,0,1], [1,0,1], [1,0,0], [0,0,0]
	// The UVs should be:
	// [0,0], [1,0], [1,1], [1,1], [0,1], [0,0]
	let local_idx = vert_idx % 6u;
	// Replace the UV calculation with:
	output.uv = vec2f(
		f32(local_idx >= 1u && local_idx <= 3u),
		f32(local_idx >= 2u && local_idx <= 4u)
	);
	
	return output;
}

// The rest of your shader can remain the same
@group(0) @binding(0) var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
// tbh only one more bind group can be used, after that it might not be supported on all devices
//@group(3) @binding(0) var<uniform> data: u32;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
	let light_dir = normalize(vec3f(0.5, 1.0, 0.5));
	let up = vec3f(0.0, 1.0, 0.0);
	
	// Combined lighting calculation
	let directional = max(dot(in.world_normal, light_dir), 0.0);
	let hemi = 0.5 + 0.5 * dot(in.world_normal, up);
	let final_light = mix(0.35 + 0.55 * directional, hemi, 0.3);
	
	let texture_color = textureSample(t_diffuse, s_diffuse, in.uv, 0);
	return vec4f(texture_color.rgb * final_light, texture_color.a);
}

