// debug_shader.wgsl - FIXED VERSION

// Line structure matching your Rust code
struct Line {
	start: vec3f,
	direction: vec3f,
}

// Bind groups
@group(0) @binding(0) var<storage> lines: array<Line>;
@group(1) @binding(0) var<uniform> camera_proj: mat4x4f;

struct VertexOutput {
	@builtin(position) position: vec4f,
	@location(0) color: vec4f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
	let line = lines[instance_index];
	let is_end = vertex_index % 2u;
	
	// Calculate world position
	let world_position = line.start + f32(is_end) * line.direction;
	
	// Apply camera transformation
	let position = camera_proj * vec4f(world_position, 1.0);
	
	var output: VertexOutput;
	output.position = position;
	output.color = vec4f(1.0, 0.0, 0.0, 1.0); // Red color for debug lines
	return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
	return input.color;
}
