struct VertexOutput {
	@builtin(position) clip_position: vec4f,
	@location(0) uv: vec2f,       // For sampling the sky texture
	@location(1) layer: i32,      // Which layer of the array to use
};

// Corrected matrix inversion implementation for WGSL
fn inverse(m: mat4x4f) -> mat4x4f {
    let a00 = m[0][0]; let a01 = m[0][1]; let a02 = m[0][2]; let a03 = m[0][3];
    let a10 = m[1][0]; let a11 = m[1][1]; let a12 = m[1][2]; let a13 = m[1][3];
    let a20 = m[2][0]; let a21 = m[2][1]; let a22 = m[2][2]; let a23 = m[2][3];
    let a30 = m[3][0]; let a31 = m[3][1]; let a32 = m[3][2]; let a33 = m[3][3];

    let b00 = a00 * a11 - a01 * a10;
    let b01 = a00 * a12 - a02 * a10;
    let b02 = a00 * a13 - a03 * a10;
    let b03 = a01 * a12 - a02 * a11;
    let b04 = a01 * a13 - a03 * a11;
    let b05 = a02 * a13 - a03 * a12;
    let b06 = a20 * a31 - a21 * a30;
    let b07 = a20 * a32 - a22 * a30;
    let b08 = a20 * a33 - a23 * a30;
    let b09 = a21 * a32 - a22 * a31;
    let b10 = a21 * a33 - a23 * a31;
    let b11 = a22 * a33 - a23 * a32;

    let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
    let inv_det = 1.0 / det;

    // Multiply each component by inv_det instead of dividing
    return mat4x4f(
        vec4f(
            (a11 * b11 - a12 * b10 + a13 * b09) * inv_det,
            (a02 * b10 - a01 * b11 - a03 * b09) * inv_det,
            (a31 * b05 - a32 * b04 + a33 * b03) * inv_det,
            (a22 * b04 - a21 * b05 - a23 * b03) * inv_det
        ),
        vec4f(
            (a12 * b08 - a10 * b11 - a13 * b07) * inv_det,
            (a00 * b11 - a02 * b08 + a03 * b07) * inv_det,
            (a32 * b02 - a30 * b05 - a33 * b01) * inv_det,
            (a20 * b05 - a22 * b02 + a23 * b01) * inv_det
        ),
        vec4f(
            (a10 * b10 - a11 * b08 + a13 * b06) * inv_det,
            (a01 * b08 - a00 * b10 - a03 * b06) * inv_det,
            (a30 * b04 - a31 * b02 + a33 * b00) * inv_det,
            (a21 * b02 - a20 * b04 - a23 * b00) * inv_det
        ),
        vec4f(
            (a11 * b07 - a10 * b09 - a12 * b06) * inv_det,
            (a00 * b09 - a01 * b07 + a02 * b06) * inv_det,
            (a31 * b01 - a30 * b03 - a32 * b00) * inv_det,
            (a20 * b03 - a21 * b01 + a22 * b00) * inv_det
        )
    );
}

@group(1) @binding(0) var<uniform> camera: mat4x4f;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
	// Fullscreen quad vertices (2 triangles)
	let positions = array<vec2f, 6>(
		vec2f(-1.0, -1.0),
		vec2f(1.0, -1.0),
		vec2f(1.0, 1.0),
		vec2f(1.0, 1.0),
		vec2f(-1.0, 1.0),
		vec2f(-1.0, -1.0)
	);
	
	let uv_coords = array<vec2f, 6>(
		vec2f(0.0, 0.0),
		vec2f(1.0, 0.0),
		vec2f(1.0, 1.0),
		vec2f(1.0, 1.0),
		vec2f(0.0, 1.0),
		vec2f(0.0, 0.0)
	);
	
	var out: VertexOutput;
	out.clip_position = vec4f(positions[vertex_index], 0.0, 1.0);
	out.uv = uv_coords[vertex_index];
	
	// Calculate which layer to use based on view direction
	let view_proj = mat4x4f(
		camera[0],
		camera[1],
		camera[2],
		vec4f(0.0, 0.0, 0.0, 1.0)
	);
	let world_pos = (inverse(view_proj) * out.clip_position).xyz;
	let direction = normalize(world_pos);
	
	// Determine which face to use based on the dominant axis
	let abs_dir = abs(direction);
	if abs_dir.x > abs_dir.y && abs_dir.x > abs_dir.z {
		if direction.x > 0.0 {
			out.layer = 0;
		} else {
			out.layer = 1;
		} // Right/Left
	} else if abs_dir.y > abs_dir.z {
		if direction.y > 0.0 {
				out.layer = 2;
			} else {
				out.layer = 3;
			} // Top/Bottom
	} else {
		if direction.z > 0.0 {
			out.layer = 4;
		} else {
			out.layer = 5;
		} // Front/Back
	}
	
	return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
	return textureSample(t_diffuse, s_diffuse, in.uv, in.layer);
}