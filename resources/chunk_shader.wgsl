struct CameraUniform {
    view_proj: mat4x4f,
};
struct ChunkUniform {
    position: u64,
};

@group(1) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var<uniform> chunk: ChunkUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
};






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

