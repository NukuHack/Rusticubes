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

fn to_bytes(pos: u32) -> vec3<f32> {
    // Unpack x, y, z (each 5 bits, stored in bits 0-14 of the u32)
    let x = f32((pos >> 0u)  & 0x1Fu);  // bits 0-4  (mask = 0b11111 or 0x1F)
    let y = f32((pos >> 5u)  & 0x1Fu);  // bits 5-9  (shift by 5, mask 0x1F)
    let z = f32((pos >> 10u) & 0x1Fu);  // bits 10-14 (shift by 10, mask 0x1F)
    // Note: Uses 15 bits total (5 bits per axis)

    return vec3<f32>(x, y, z);
}
fn to_normal(pos :u32) -> u32 {
    return pos >> 16u;
}

struct VertexInput {
    @location(0) packed_data: u32,
    
    //@location(2) uv: vec2f,
};
/*
// normal's ordering
const CUBE_FACES: [Vec3; 6] = [
    Vec3::NEG_X, // [0] Left face
    Vec3::X,     // [1] Right face  
    Vec3::NEG_Z, // [2] Front face
    Vec3::Z,     // [3] Back face
    Vec3::Y,     // [4] Top face
    Vec3::NEG_Y, // [5] Bottom face
];
*/

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
};