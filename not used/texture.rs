use wgpu::{Device, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

fn create_texture_array(
    device: &Device,
    width: u32,
    height: u32,
    array_size: u32,
    format: TextureFormat,
    label: Option<&str>,
) -> wgpu::Texture {
    device.create_texture(&TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: array_size,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

use wgpu::{SamplerDescriptor, TextureViewDescriptor};

// Create view
let view = texture.create_view(&TextureViewDescriptor {
    dimension: Some(wgpu::TextureViewDimension::D2Array),
    ..Default::default()
});

// Create sampler
let sampler = device.create_sampler(&SamplerDescriptor {
    label: Some("texture_array_sampler"),
    address_mode_u: wgpu::AddressMode::ClampToEdge,
    address_mode_v: wgpu::AddressMode::ClampToEdge,
    address_mode_w: wgpu::AddressMode::ClampToEdge,
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Linear,
    mipmap_filter: wgpu::FilterMode::Linear,
    ..Default::default()
});

let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("texture_array_bind_group_layout"),
    entries: &[
        // Texture array
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        // Sampler
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ],
});

let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("texture_array_bind_group"),
    layout: &bind_group_layout,
    entries: &[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&sampler),
        },
    ],
});

const SHADER: &str = "

@group(0) @binding(0) var texture_array: texture_2d_array<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

fn sample_texture_array(uv: vec2<f32>, index: u32) -> vec4<f32> {
    return textureSample(texture_array, tex_sampler, uv, index);
}

@fragment
fn fs_main(
    @location(0) uv: vec2<f32>,
    @location(1) texture_index: u32,
) -> @location(0) vec4<f32> {
    return sample_texture_array(uv, texture_index);
}

";