use crate::traits::VectorTypeConversion;
use cgmath::{InnerSpace, SquareMatrix, Vector3, Vector4};
use image::GenericImageView;
use std::mem;
use wgpu::util::DeviceExt;

// --- Vertex & Buffer Layouts ---
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    // this is 256 bits ... too much
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// --- Geometry Buffer (modified for chunk meshes) ---
#[derive(Debug, Clone)]
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
}

impl GeometryBuffer {
    pub fn new(device: &wgpu::Device, indices: &[u16], vertices: &[Vertex]) -> Self {
        // Handle empty geometry case
        if vertices.is_empty() || indices.is_empty() {
            return Self::empty(device);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            num_vertices: vertices.len() as u32,
        }
    }

    pub fn empty(device: &wgpu::Device) -> Self {
        // Create minimal buffers that won't cause rendering issues
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Empty Vertex Buffer"),
            size: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Empty Index Buffer"),
            size: std::mem::size_of::<u16>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: 0,
            num_vertices: 0,
        }
    }
}

pub fn add_def_cube() {
    unsafe {
        let state = super::get_state();

        // Calculate where to place the cube (in front of the camera)
        let placement_distance = 6.0; // Distance in front of camera
        let placement_position = (state.camera_system.camera.position
            + state.camera_system.camera.forward() * placement_distance)
            .to_vec3_i32();

        // Create cube at the correct position
        let cube = super::cube::Block::new();
        state.data_system.world.set_block(placement_position, cube);

        // Update the chunk's mesh
        let chunk_pos = super::cube::ChunkCoord::from_world_pos(placement_position);
        if let Some(chunk) = state.data_system.world.get_chunk_mut(chunk_pos) {
            chunk.make_mesh(super::get_state().device(), chunk_pos, true);
        }
    }
}
pub fn add_def_chunk() {
    unsafe {
        let state = super::get_state();
        let chunk_pos = (state.camera_system.camera.position).to_vec3_i32();
        let chunk_pos_c_c = super::cube::ChunkCoord::from_world_pos(chunk_pos);

        if state
            .data_system
            .world
            .loaded_chunks
            .contains(&chunk_pos_c_c)
        {
            return;
            // why would you try to load a chunk what already exist ?
        }

        if state.data_system.world.load_chunk(chunk_pos_c_c) {
            // Get the chunk and update its mesh
            if let Some(chunk) = state.data_system.world.get_chunk_mut(chunk_pos_c_c) {
                chunk.make_mesh(super::get_state().device(), chunk_pos_c_c, true);
            }
        } else {
            eprintln!("Chunk load failed at: {:?}", chunk_pos_c_c);
        }
    }
}
pub fn add_full_world() {
    unsafe {
        let state = super::get_state();
        let chunk_pos = (state.camera_system.camera.position).to_vec3_i32();
        state.data_system.world.update_loaded_chunks(chunk_pos, 5);
        state
            .data_system
            .world
            .make_chunk_meshes(super::get_state().device());
    }
}

pub fn cast_ray_and_select_block(
    camera: &super::camera::Camera,
    projection: &super::camera::Projection,
    world: &super::cube::World,
    max_distance: f32,
) -> Option<Vector3<i32>> {
    let ray_clip = Vector4::new(0.0, 0.0, -1.0, 1.0);
    let inv_proj = projection.calc_matrix().invert().unwrap();
    let mut ray_eye: Vector4<f32> = inv_proj * ray_clip;
    ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);

    let inv_view = camera.calc_matrix().invert().unwrap();
    let ray_world = inv_view * ray_eye;
    let ray_dir = ray_world.truncate().normalize();
    let ray_origin = camera.position;

    let steps = (max_distance * 2.0) as usize;
    let step_size = max_distance / steps as f32;

    for i in 0..steps {
        let t = i as f32 * step_size;
        let current_pos = ray_origin + ray_dir * t;
        let block_pos = Vector3::new(
            current_pos.x.floor() as i32,
            current_pos.y.floor() as i32,
            current_pos.z.floor() as i32,
        );

        if let Some(block) = world.get_block(block_pos) {
            if !block.is_empty() {
                return Some(block_pos);
            }
        }
    }

    None
}

pub fn rem_raycasted_block() {
    let block_pos = unsafe {
        let state = super::get_state();
        cast_ray_and_select_block(
            &state.camera_system.camera,
            &state.camera_system.projection,
            &state.data_system.world,
            10.0,
        )
    };

    if let Some(block_pos) = block_pos {
        unsafe {
            let state = super::get_state();

            // Remove the block
            state.data_system.world.set_block(
                block_pos,
                super::cube::Block::default(), // default is air
            );

            // Update the chunk's mesh
            let chunk_pos = super::cube::ChunkCoord::from_world_pos(block_pos);
            if let Some(chunk) = state.data_system.world.get_chunk_mut(chunk_pos) {
                chunk.make_mesh(super::get_state().device(), chunk_pos, true);
            }
        }
    }
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> std::result::Result<Self, image::ImageError> {
        let img: image::DynamicImage = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> std::result::Result<Self, image::ImageError> {
        let rgba: image::RgbaImage = img.to_rgba8();
        let dimensions: (u32, u32) = img.dimensions();

        let size: wgpu::Extent3d = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture: wgpu::Texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view: wgpu::TextureView = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size: wgpu::Extent3d = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };

        let texture: wgpu::Texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view: wgpu::TextureView = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            }),
        }
    }
}

// --- Texture Manager ---
pub struct TextureManager {
    pub texture: Texture,
    pub depth_texture: Texture,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureManager {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let texture_path = "resources/cube-diffuse.jpg";
        let bytes = std::fs::read(texture_path).expect("Texture not found");
        let texture = Texture::from_bytes(device, queue, &bytes, texture_path).unwrap();

        let depth_texture = Texture::create_depth_texture(device, config, "Depth Texture");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some("Texture Bind Group Layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("Texture Bind Group"),
        });
        Self {
            texture,
            depth_texture,
            bind_group,
            bind_group_layout,
        }
    }
}
