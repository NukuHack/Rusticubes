
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    // Mark the position with @builtin(position)
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
@group(0) @binding(1) var font_texture: texture_2d<f32>;

struct FragmentInput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let sampled_color = textureSample(font_texture, font_sampler, in.uv);
    return in.color;
}