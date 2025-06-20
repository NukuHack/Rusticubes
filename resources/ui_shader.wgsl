
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = in.uv;
    out.color = in.color;
    // Assign to the position field instead of gl_Position
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    return out;
}
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
// Helper function to extract 2 numbers from a packed u32
fn unpack_number(packed: u32) -> vec2<u32> {
    return vec2<u32>(
        packed & 0xFFFFu,         // one number (lower 16 bits)
        (packed >> 16) & 0xFFFFu  // other number (upper 16 bits)
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
    
    // Calculate adjusted progress (0 when below hold, linear when above)
    let adjusted_progress = clamp((raw_progress - hold_pct) / (1.0 - hold_pct), 0.0, 1.0);
    
    if (adjusted_progress == 0.0) {
        // Display current frame only when below hold threshold
        return sample_frame(in.uv, frames.x) * in.color;
    } else {
        // Alpha-weighted blend when in transition phase
        let current = sample_frame(in.uv, frames.x);
        let next = sample_frame(in.uv, frames.y);
        
        let current_col = current * in.color * (1.0 - adjusted_progress);
        let next_col = next * in.color * adjusted_progress;
        
        return current_col + next_col;
    }
}