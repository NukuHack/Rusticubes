// Vertex shader
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

// Fragment shader
@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var screen_sampler: sampler;

// Try different values for these constants
const FXAA_REDUCE_MIN = 1.0/128.0;  // Lower = more AA but blurrier
const FXAA_REDUCE_MUL = 1.0/8.0;     // Higher = more AA but blurrier
const FXAA_SPAN_MAX = 8.0;           // Higher = more AA but slower

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(screen_texture));
    let rgb_nw = textureSample(screen_texture, screen_sampler, in.tex_coord + vec2<f32>(-1.0, -1.0) * texel_size).rgb;
    let rgb_ne = textureSample(screen_texture, screen_sampler, in.tex_coord + vec2<f32>(1.0, -1.0) * texel_size).rgb;
    let rgb_sw = textureSample(screen_texture, screen_sampler, in.tex_coord + vec2<f32>(-1.0, 1.0) * texel_size).rgb;
    let rgb_se = textureSample(screen_texture, screen_sampler, in.tex_coord + vec2<f32>(1.0, 1.0) * texel_size).rgb;
    let rgb_m = textureSample(screen_texture, screen_sampler, in.tex_coord).rgb;
    
    let luma_nw = rgb_to_luma(rgb_nw);
    let luma_ne = rgb_to_luma(rgb_ne);
    let luma_sw = rgb_to_luma(rgb_sw);
    let luma_se = rgb_to_luma(rgb_se);
    let luma_m = rgb_to_luma(rgb_m);
    
    let luma_min = min(luma_m, min(min(luma_nw, luma_ne), min(luma_sw, luma_se)));
    let luma_max = max(luma_m, max(max(luma_nw, luma_ne), max(luma_sw, luma_se)));
    
    let dir_o = vec2<f32>(
        -((luma_nw + luma_ne) - (luma_sw + luma_se)),
        ((luma_nw + luma_sw) - (luma_ne + luma_se))
    );
    
    let dir_reduce = max((luma_nw + luma_ne + luma_sw + luma_se) * (0.25 * FXAA_REDUCE_MUL), FXAA_REDUCE_MIN);
    let rcp_dir_min = 1.0 / (min(abs(dir_o.x), abs(dir_o.y)) + dir_reduce);
    
    let dir = min(vec2<f32>(FXAA_SPAN_MAX), max(vec2<f32>(-FXAA_SPAN_MAX), dir_o * rcp_dir_min)) * texel_size;
    
    let rgb_a = 0.5 * (
        textureSample(screen_texture, screen_sampler, in.tex_coord + dir * (1.0/3.0 - 0.5)).rgb +
        textureSample(screen_texture, screen_sampler, in.tex_coord + dir * (2.0/3.0 - 0.5)).rgb
    );
    
    let rgb_b = rgb_a * 0.5 + 0.25 * (
        textureSample(screen_texture, screen_sampler, in.tex_coord + dir * -0.5).rgb +
        textureSample(screen_texture, screen_sampler, in.tex_coord + dir * 0.5).rgb
    );
    
    let luma_b = rgb_to_luma(rgb_b);
    
    if (luma_b < luma_min) || (luma_b > luma_max) {
        return vec4<f32>(rgb_a, 1.0);
    } else {
        return vec4<f32>(rgb_b, 1.0);
    }
}

fn rgb_to_luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}