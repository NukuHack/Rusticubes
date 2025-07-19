struct VertexOutput {
	@builtin(position) clip_position: vec4f,
	@location(0) uv: vec2f,       // For sampling the sky texture
};

@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct CameraUniform {
	view_proj: mat4x4f,
	pos: vec3f,
};

// Combined UV coordinates with face offsets for sideways T-shaped cubemap texture
// Each face's UVs are already scaled (0.25 x 0.333333) and offset to correct positions
const FACE_UV_COORDS = array<vec2f, 36>(
	// Front face 
	vec2f(0.5, 0.666667), vec2f(0.75, 0.333333), vec2f(0.75, 0.666667),
	vec2f(0.75, 0.333333), vec2f(0.5, 0.666667), vec2f(0.5, 0.333333),
	// Back face 
	vec2f(0.25, 0.666667), vec2f(0.0, 0.333333), vec2f(0.25, 0.333333),
	vec2f(0.0, 0.333333), vec2f(0.25, 0.666667), vec2f(0.0, 0.666667),
	// Top face 
    vec2f(1.0, 0.0), vec2f(0.75, 0.333333), vec2f(0.75, 0.0),
    vec2f(0.75, 0.333333), vec2f(1.0, 0.0), vec2f(1.0, 0.333333),
	// Bottom face 
	vec2f(0.75, 0.666667), vec2f(1.0, 1.0), vec2f(1.0, 0.666667),
	vec2f(1.0, 1.0), vec2f(0.75, 0.666667), vec2f(0.75, 1.0),
	// Right face 
	vec2f(1.0, 0.666667), vec2f(0.75, 0.333333), vec2f(1.0, 0.333333),
	vec2f(0.75, 0.333333), vec2f(1.0, 0.666667), vec2f(0.75, 0.666667),
	// Left face 
	vec2f(0.25, 0.666667), vec2f(0.5, 0.333333), vec2f(0.5, 0.666667),
	vec2f(0.5, 0.333333), vec2f(0.25, 0.666667), vec2f(0.25, 0.333333)
);
const VERTEX_POSITIONS = array<vec3f, 36>(
	// Front face
	vec3f(-1.0, -1.0,  1.0), vec3f( 1.0,  1.0,  1.0), vec3f( 1.0, -1.0,  1.0),
	vec3f( 1.0,  1.0,  1.0), vec3f(-1.0, -1.0,  1.0), vec3f(-1.0,  1.0,  1.0),
	// Back face
	vec3f(-1.0, -1.0, -1.0), vec3f( 1.0,  1.0, -1.0), vec3f(-1.0,  1.0, -1.0),
	vec3f( 1.0,  1.0, -1.0), vec3f(-1.0, -1.0, -1.0), vec3f( 1.0, -1.0, -1.0),
	// Top face
	vec3f(-1.0,  1.0, -1.0), vec3f( 1.0,  1.0,  1.0), vec3f(-1.0,  1.0,  1.0),
	vec3f( 1.0,  1.0,  1.0), vec3f(-1.0,  1.0, -1.0), vec3f( 1.0,  1.0, -1.0),
	// Bottom face
	vec3f(-1.0, -1.0, -1.0), vec3f( 1.0, -1.0,  1.0), vec3f( 1.0, -1.0, -1.0),
	vec3f( 1.0, -1.0,  1.0), vec3f(-1.0, -1.0, -1.0), vec3f(-1.0, -1.0,  1.0),
	// Right face
	vec3f( 1.0, -1.0, -1.0), vec3f( 1.0,  1.0,  1.0), vec3f( 1.0,  1.0, -1.0),
	vec3f( 1.0,  1.0,  1.0), vec3f( 1.0, -1.0, -1.0), vec3f( 1.0, -1.0,  1.0),
	// Left face
	vec3f(-1.0, -1.0, -1.0), vec3f(-1.0,  1.0,  1.0), vec3f(-1.0, -1.0,  1.0),
	vec3f(-1.0,  1.0,  1.0), vec3f(-1.0, -1.0, -1.0), vec3f(-1.0,  1.0, -1.0)
);

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
	// Cube vertices (36 vertices - 6 faces × 2 triangles × 3 vertices) with CW winding

	var out: VertexOutput;
	let world_pos = VERTEX_POSITIONS[vertex_index];
	
	// Position the skybox cube at the camera position
	let skybox_world_pos = world_pos + camera.pos;
	
	// Transform with the full view-projection matrix
	out.clip_position = camera.view_proj * vec4f(skybox_world_pos, 1.0);
	
	// Set z to w for maximum depth (skybox renders behind everything)
	out.clip_position.z = out.clip_position.w;
	
	out.uv = FACE_UV_COORDS[vertex_index];
	
	return out;
}

@group(0) @binding(0) var s_diffuse: sampler;
@group(0) @binding(1) var t_diffuse: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
	return textureSample(t_diffuse, s_diffuse, in.uv);
}