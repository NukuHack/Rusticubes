// Vertex and texture utilities for wgpu rendering.
use crate::ext::rs;
/// Standard format for depth textures
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Creates a depth texture for rendering
pub fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    label: &str,
) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: config.width.max(1),
        height: config.height.max(1),
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    texture
}

// --- Texture Manager ---

/// Manages all texture resources for rendering
pub struct TextureManager {
    depth_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    render_texture: wgpu::Texture,
    render_texture_view: wgpu::TextureView,
    post_processing_bind_group: wgpu::BindGroup,
}

impl TextureManager {
    /// Creates a new texture manager with all required resources
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let depth_texture = create_depth_texture(device, config, "Depth Texture");

        let (render_texture, render_texture_view) = create_render_texture(device, config);
        let post_processing_bind_group = create_post_processing_bind_group(device, &render_texture_view);

        // Create resources
        let bind_group_layout = create_texture_array_bind_group_layout(&device);
        let paths = rs::find_png_resources("blocks");
        let (_array_texture, array_texture_view) = create_texture_array(&device, &queue, &paths).unwrap();
        let array_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = create_texture_array_bind_group(&device, &bind_group_layout, &array_texture_view, &array_sampler);

        Self {
            depth_texture,
            bind_group,
            bind_group_layout,
            render_texture,
            render_texture_view,
            post_processing_bind_group,
        }
    }

    #[inline]
    pub fn depth_texture(&self) -> &wgpu::Texture {
        &self.depth_texture
    }
    #[inline]
    pub fn depth_texture_mut(&mut self) -> &mut wgpu::Texture {
        &mut self.depth_texture
    }
    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    #[inline]
    pub fn render_texture(&self) -> &wgpu::Texture {
        &self.render_texture
    }
    #[inline]
    pub fn render_texture_view(&self) -> &wgpu::TextureView {
        &self.render_texture_view
    }
    #[inline]
    pub fn post_processing_bind_group(&self) -> &wgpu::BindGroup {
        &self.post_processing_bind_group
    }
}

// Creates a bind group layout suitable for a 2D texture array
fn create_texture_array_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture_array_bind_group_layout"),
        entries: &[
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
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}


/// Creates a texture array from a list of image paths.
/// Only images with matching dimensions will be included in the array.
/// Returns the texture, its view, and the actual number of layers loaded.
fn create_texture_array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image_paths: &[String],
) -> Option<(wgpu::Texture, wgpu::TextureView)> {
    if image_paths.is_empty() {
        println!("No image paths provided");
        return None;
    }

    // Load first image to get dimensions
    let (first_image_data, base_width, base_height) = match rs::load_image_from_path(&image_paths[0]) {
        Some(data) => data,
        None => {
            println!("Failed to load first image: {}", image_paths[0]);
            return None;
        }
    };

    // Collect all valid images with matching dimensions
    let mut valid_images = vec![first_image_data];
    let mut valid_count = 1;
    
    for path in image_paths.iter().skip(1) {
        match rs::load_image_from_path(path) {
            Some((data, width, height)) if width == base_width && height == base_height => {
                valid_images.push(data);
                valid_count += 1;
            }
            Some((_, width, height)) => {
                println!("Image dimensions don't match for: {}, got ({}, {}) instead of ({}, {})",
                    path, width, height, base_width, base_height);
            }
            None => {
                println!("Failed to load image: {}", path);
            }
        }
    }

    if valid_count == 0 {
        println!("No valid images found");
        return None;
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture_array"),
        size: wgpu::Extent3d {
            width: base_width,
            height: base_height,
            depth_or_array_layers: valid_count as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Fill each layer with its corresponding image
    for (layer, image_data) in valid_images.into_iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
                aspect: wgpu::TextureAspect::All,
            },
            &image_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * base_width),
                rows_per_image: Some(base_height),
            },
            wgpu::Extent3d {
                width: base_width,
                height: base_height,
                depth_or_array_layers: 1,
            },
        );
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        base_array_layer: 0,
        array_layer_count: Some(valid_count as u32),
        ..Default::default()
    });

    Some((texture, view))
}

// Creates a bind group for the texture array
fn create_texture_array_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("texture_array_bind_group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn create_render_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Render Texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_post_processing_bind_group(
    device: &wgpu::Device,
    render_texture_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Post Processing Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Post Processing Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Post Processing Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(render_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}
