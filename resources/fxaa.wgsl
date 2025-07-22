// Vertex shader remains the same
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Full-screen triangle
    out.tex_coord = vec2<f32>(
        f32(vertex_index >> 1u) * 2.0,
        f32(vertex_index & 1u) * 2.0
    );
    out.position = vec4<f32>(out.tex_coord * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// Fragment shader optimizations
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

// Tuned constants for better quality/performance balance
const FXAA_REDUCE_MIN = 1.0/128.0;
const FXAA_REDUCE_MUL = 1.0/12.0;  // Slightly reduced for sharper results
const FXAA_SPAN_MAX = 6.0;         // Reduced for better performance

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(screen_texture));
    let uv = in.tex_coord;
    
    // Sample with offsets in one operation for better cache utilization
    let nw = uv + vec2<f32>(-1.0, -1.0) * texel_size;
    let ne = uv + vec2<f32>(1.0, -1.0) * texel_size;
    let sw = uv + vec2<f32>(-1.0, 1.0) * texel_size;
    let se = uv + vec2<f32>(1.0, 1.0) * texel_size;
    
    let rgb_nw = textureSample(screen_texture, screen_sampler, nw).rgb;
    let rgb_ne = textureSample(screen_texture, screen_sampler, ne).rgb;
    let rgb_sw = textureSample(screen_texture, screen_sampler, sw).rgb;
    let rgb_se = textureSample(screen_texture, screen_sampler, se).rgb;
    let rgb_m = textureSample(screen_texture, screen_sampler, uv).rgb;
    
    // Precompute common sums
    let luma_nw = dot(rgb_nw, vec3<f32>(0.299, 0.587, 0.114));
    let luma_ne = dot(rgb_ne, vec3<f32>(0.299, 0.587, 0.114));
    let luma_sw = dot(rgb_sw, vec3<f32>(0.299, 0.587, 0.114));
    let luma_se = dot(rgb_se, vec3<f32>(0.299, 0.587, 0.114));
    let luma_m = dot(rgb_m, vec3<f32>(0.299, 0.587, 0.114));
    
    let luma_min = min(luma_m, min(min(luma_nw, luma_ne), min(luma_sw, luma_se)));
    let luma_max = max(luma_m, max(max(luma_nw, luma_ne), max(luma_sw, luma_se)));
    
    // Compute edge direction with precomputed sums
    let luma_sum_nwne = luma_nw + luma_ne;
    let luma_sum_swse = luma_sw + luma_se;
    let dir_o = vec2<f32>(
        -(luma_sum_nwne - luma_sum_swse),
        (luma_nw + luma_sw) - (luma_ne + luma_se)
    );
    
    let dir_reduce = max((luma_sum_nwne + luma_sum_swse) * (0.25 * FXAA_REDUCE_MUL), FXAA_REDUCE_MIN);
    let rcp_dir_min = 1.0 / (min(abs(dir_o.x), abs(dir_o.y)) + dir_reduce);
    
    let dir = clamp(dir_o * rcp_dir_min, vec2<f32>(-FXAA_SPAN_MAX), vec2<f32>(FXAA_SPAN_MAX)) * texel_size;
    
    // Optimized sampling pattern
    let offset1 = dir * (1.0/3.0 - 0.5);
    let offset2 = dir * (2.0/3.0 - 0.5);
    let rgb_a = 0.5 * (
        textureSample(screen_texture, screen_sampler, uv + offset1).rgb +
        textureSample(screen_texture, screen_sampler, uv + offset2).rgb
    );
    
    let offset3 = dir * -0.5;
    let offset4 = dir * 0.5;
    let rgb_b = rgb_a * 0.5 + 0.25 * (
        textureSample(screen_texture, screen_sampler, uv + offset3).rgb +
        textureSample(screen_texture, screen_sampler, uv + offset4).rgb
    );
    
    let luma_b = dot(rgb_b, vec3<f32>(0.299, 0.587, 0.114));
    if (luma_b < luma_min || luma_b > luma_max) {
        return vec4<f32>(rgb_a,1.0);
    } else {
        return vec4<f32>(rgb_b,1.0);
    }
}