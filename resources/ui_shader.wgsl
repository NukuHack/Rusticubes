struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) packed_data: u32, // This matches your Rust struct
};

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

// Your existing unpack_color function
fn unpack_color(packed: u32) -> vec4<f32> {
    return vec4(
        f32((packed >> 24) & 0xFF) / 255.0,
        f32((packed >> 16) & 0xFF) / 255.0,
        f32((packed >> 8) & 0xFF) / 255.0,
        f32(packed & 0xFF) / 255.0
    );
}

@vertex
fn vs_main(
    in: VertexInput,
    @builtin(vertex_index) vert_idx: u32  // Built-in vertex index
) -> VertexOutput {
    var out: VertexOutput;
    
    // Calculate UVs based on vertex index (for a quad made of two triangles)
    out.uv = vec2<f32>(
        select(0.0, 1.0, (vert_idx & 1u) == 1u),  // U (horizontal) coordinate
        select(1.0, 0.0, (vert_idx & 2u) == 2u)   // V (vertical) coordinate
    );
    
    out.color = unpack_color(in.packed_data);
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    return out;
}

// The rest of your shader can remain the same
@group(0) @binding(0) var font_sampler: sampler;
@group(0) @binding(1) var texture_array: texture_2d_array<f32>;
struct Uniforms {
    frame_data: u32, // Packed : lower half = current frame, upper half = next frame
    progress_data: u32,  // Packed : lower half = progress, upper half = blend delay
};
@group(1) @binding(0) var<uniform> data: Uniforms;

struct FragmentInput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};
fn sample_frame(uv: vec2<f32>, frame: u32) -> vec4<f32> {
    return textureSample(texture_array, font_sampler, uv, frame);
}
fn unpack_number(packed: u32) -> vec2<u32> {
    return vec2<u32>(
        packed & 0xFFFFu,
        (packed >> 16) & 0xFFFFu
    );
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // For solid color rectangles
    if (in.uv.x == 0.0 && in.uv.y == 0.0) {
        return in.color;
    }
    
    let frames = unpack_number(data.frame_data);
    let progress_info = unpack_number(data.progress_data);
    let raw_progress = f32(progress_info.x) / 100.0;
    let hold_pct = f32(progress_info.y) / 100.0;
    
    let adjusted_progress = clamp((raw_progress - hold_pct) / (1.0 - hold_pct), 0.0, 1.0);
    
    if (adjusted_progress == 0.0) {
        return sample_frame(in.uv, frames.x) * in.color;
    } else {
        let current = sample_frame(in.uv, frames.x);
        let next = sample_frame(in.uv, frames.y);
        
        let current_col = current * in.color * (1.0 - adjusted_progress);
        let next_col = next * in.color * adjusted_progress;
        
        return current_col + next_col;
    }
}