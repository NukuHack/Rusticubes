use super::geometry;
use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Rotation3, Zero};
use image::GenericImageView;
use std::{env, mem, path, result};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
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
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TexCoord {
    pub uv: [f32; 2],
}

impl TexCoord {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[allow(dead_code, unused, unused_attributes)]
// Update GeometryBuffer to include texture coordinate buffer
pub struct Cube {
    pub vertices: [Vertex; 8],
    pub indices: [u32; 36],
    pub texture_coords: [TexCoord; 8],
}

impl Cube {
    pub fn default() -> Self {
        const VERTICES: [Vertex; 8] = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [0.0, 1.0, -1.0],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [1.0, 1.0, -1.0],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [1.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
            },
        ];

        const INDICES: [u32; 36] = [
            1, 0, 2, 3, 2, 0, // Front face
            4, 5, 6, 6, 7, 4, // Back face
            0, 4, 7, 3, 0, 7, // Bottom
            5, 1, 6, 1, 2, 6, // Top
            6, 2, 7, 2, 3, 7, // Right
            4, 0, 5, 0, 1, 5, // Left
        ];

        const TEXTURE_COORDS: [TexCoord; 8] = [
            TexCoord { uv: [0.0, 0.0] }, // Front
            TexCoord { uv: [0.0, 1.0] },
            TexCoord { uv: [1.0, 1.0] },
            TexCoord { uv: [1.0, 0.0] },
            TexCoord { uv: [0.0, 0.0] }, // Back
            TexCoord { uv: [0.0, 1.0] },
            TexCoord { uv: [1.0, 1.0] },
            TexCoord { uv: [1.0, 0.0] },
        ];

        Self {
            vertices: VERTICES,
            indices: INDICES,
            texture_coords: TEXTURE_COORDS,
        }
    }
}

pub struct CubeBuffer;

impl CubeBuffer {
    pub fn new(
        device: &wgpu::Device,
        cube: &geometry::Cube,
    ) -> geometry::GeometryBuffer {
        geometry::GeometryBuffer::new(
            &device,
            &cube.indices,
            &cube.vertices,
            &cube.texture_coords,
        )
    }
}

#[allow(dead_code, unused, unused_attributes)]
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture_coord_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
}

impl GeometryBuffer {
    pub fn new(
        device: &wgpu::Device,
        indices: &[u32],
        vertices: &[geometry::Vertex], // Use `Self` prefix for clarity
        texture_coords: &[geometry::TexCoord], // Use `Self` prefix for clarity
    ) -> Self {
        let vertex_buffer: wgpu::Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let texture_coord_buffer: wgpu::Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Texture Coordinate Buffer"),
                contents: bytemuck::cast_slice(texture_coords),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer: wgpu::Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            vertex_buffer,
            index_buffer,
            texture_coord_buffer,
            num_indices: indices.len() as u32,
            num_vertices: vertices.len() as u32,
        }
    }
}

#[allow(dead_code, unused)]
pub struct InstanceManager {
    pub instances: Vec<Instance>,
    pub instance_data: Vec<InstanceRaw>,
    pub instance_buffer: wgpu::Buffer,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device) -> Self {
        const SPACE_BETWEEN: f32 = 3.0;
        const NUM_INSTANCES: u32 = 2;
        let instances: Vec<geometry::Instance> = (0..NUM_INSTANCES)
            .flat_map(|z| {
                (0..NUM_INSTANCES).map(move |x| {
                    let position: cgmath::Vector3<f32> = cgmath::Vector3 {
                        x: SPACE_BETWEEN * (x as f32 - NUM_INSTANCES as f32 / 2.0),
                        y: 0.0,
                        z: SPACE_BETWEEN * (z as f32 - NUM_INSTANCES as f32 / 2.0),
                    };

                    let rotation: cgmath::Quaternion<f32> = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    geometry::Instance { position, rotation }
                })
            })
            .collect();
        let instance_data: Vec<geometry::InstanceRaw> =
            instances.iter().map(geometry::Instance::to_raw).collect();
        let instance_buffer: wgpu::Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self {
            instances,
            instance_data,
            instance_buffer,
        }
    }
	
    pub fn add_custom_instance(
        &mut self,
        position: cgmath::Vector3<f32>,
        rotation: cgmath::Quaternion<f32>,
        device: &wgpu::Device,
    ) {
        // Create the new instance
        let new_instance = geometry::Instance { position, rotation };

        // Convert the new instance to raw data
        let new_instance_raw = new_instance.to_raw();

        // Add the new instance to the list
        self.instances.push(new_instance);

        // Append the raw data to the instance data buffer
        self.instance_data.push(new_instance_raw);

        // Create a new buffer with the updated instance data
        let new_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("New Instance Buffer"),
            contents: bytemuck::cast_slice(&[new_instance_raw]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Replace the old instance buffer with the new one
        self.instance_buffer = new_instance_buffer;
    }
}


pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        let matrix =
            cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation);
        InstanceRaw {
            model: matrix.into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Texture {
    #[allow(unused)]
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
    ) -> result::Result<Self, image::ImageError> {
        let img: image::DynamicImage = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> result::Result<Self, image::ImageError> {
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

    pub fn load_texture_bytes(path: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
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

#[allow(dead_code, unused, unused_variables)]
pub struct TextureManager {
    pub texture: Texture,
    pub depth_texture: Texture,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub path: String,
}

impl TextureManager {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let current_dir: path::PathBuf =
            env::current_dir().expect("Failed to get current directory");
        println!("Current directory: {:?}", current_dir);

        let raw_path: &str = r"cube-diffuse.jpg";

        let full_path: path::PathBuf = current_dir.join("resources").join(raw_path);
        let path: &str = full_path.to_str().expect("Path contains invalid UTF-8");

        let bytes: Vec<u8> =
            Texture::load_texture_bytes(path).expect("Failed to load texture bytes");

        let texture: Texture =
            Texture::from_bytes(device, queue, &bytes, path).expect("Failed to load texture");

        let depth_texture: Texture = Texture::create_depth_texture(device, config, "depth_texture");

        let bind_group_layout: wgpu::BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("texture_bind_group_layout"),
            });

        let bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            label: Some("diffuse_bind_group"),
        });

        Self {
            texture,
            depth_texture,
            bind_group,
            bind_group_layout,
            path: raw_path.to_string(),
        }
    }
}
