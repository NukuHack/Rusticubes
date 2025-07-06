// Fragment shader: PBR with rust and metal blending.
struct FragmentInput {
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) tangent: vec3f,
};

// Textures
@group(1) @binding(0) var albedo_map: texture_2d<f32>;
@group(1) @binding(1) var normal_map: texture_2d<f32>;
@group(1) @binding(2) var metallic_roughness_map: texture_2d<f32>;
@group(1) @binding(3) var sampler: sampler;

// Lights (simplified: single directional light)
struct Light {
    direction: vec3f,
    color: vec3f,
};
@group(2) @binding(0) var<uniform> light: Light;

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4f {
    // Sample textures
    let albedo = textureSample(albedo_map, sampler, input.uv).rgb;
    let normal_map_rgb = textureSample(normal_map, sampler, input.uv).rgb;
    let metallic_roughness = textureSample(metallic_roughness_map, sampler, input.uv).rgb;

    // Extract metallic/roughness (R = metallic, G = roughness)
    let metallic = metallic_roughness.r;
    let roughness = metallic_roughness.g;

    // Calculate TBN matrix (for normal mapping)
    let bitangent = normalize(cross(input.normal, input.tangent));
    let tbn = mat3x3f(input.tangent, bitangent, input.normal);
    let tangent_space_normal = normalize(normal_map_rgb * 2.0 - 1.0);
    let world_normal = normalize(tbn * tangent_space_normal);

    // View and light vectors
    let view_dir = normalize(camera_pos - input.world_position);
    let light_dir = normalize(-light.direction);

    // PBR Lighting calculations (simplified)
    // -- Diffuse (Lambertian)
    let diffuse = max(dot(world_normal, light_dir), 0.0) * light.color;

    // -- Specular (Cook-Torrance approximation)
    let halfway_dir = normalize(light_dir + view_dir);
    let specular_strength = pow(max(dot(world_normal, halfway_dir), 0.0), 32.0);
    let specular = specular_strength * light.color;

    // Blend rust (non-metal) and metal
    let dielectric_fresnel = 0.04; // Base reflectance for non-metals
    let reflectance = mix(dielectric_fresnel, albedo, metallic);
    let final_color = (albedo * diffuse + specular * reflectance) * (1.0 - roughness);

    return vec4f(final_color, 1.0);
}



// What’s Missing (For Brevity)
// IBL (Image-Based Lighting): Skipped for simplicity.
// Shadow Maps: Not included here.
// Occlusion: AO would multiply the final color.