use super::geometry;
use bytemuck::{Pod, Zeroable};
use cgmath::InnerSpace;
use cgmath::Rotation3;
use image::GenericImageView;
use std::{mem, result};
use wgpu::util::DeviceExt;

// --- Vertex & Buffer Layouts ---
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
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

#[allow(dead_code, unused, unused_attributes)]
// --- Cube Geometry ---
pub struct Cube {
    pub vertices: [Vertex; 8],
    pub indices: [u32; 36],
}

impl Cube {
    pub fn default() -> Self {
        const VERTICES: [Vertex; 8] = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                uv: [1.0, 0.0],
            },
        ];

        const INDICES: [u32; 36] = [
            1, 0, 2, 3, 2, 0, // Front face (z=0)
            4, 5, 6, 6, 7, 4, // Back face (z=-1)
            0, 4, 7, 3, 0, 7, // Bottom (y=0)
            5, 1, 6, 1, 2, 6, // Top (y=1)
            6, 2, 7, 2, 3, 7, // Right (x=1)
            4, 0, 5, 0, 1, 5, // Left (x=0)
        ];

        Self {
            vertices: VERTICES,
            indices: INDICES,
        }
    }
}

pub struct CubeBuffer;

impl CubeBuffer {
    pub fn new(device: &wgpu::Device, cube: &geometry::Cube) -> geometry::GeometryBuffer {
        geometry::GeometryBuffer::new(&device, &cube.indices, &cube.vertices)
    }
}

#[allow(dead_code, unused, unused_attributes)]
// --- Geometry Buffer ---
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl GeometryBuffer {
    pub fn new(device: &wgpu::Device, indices: &[u32], vertices: &[Vertex]) -> Self {
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
        }
    }
}

#[allow(dead_code, unused)]
// --- Instance Manager ---
pub struct InstanceManager {
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
    pub capacity: usize,
}

impl InstanceManager {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        num_instances: u32,
        space_between: f32,
        do_default: bool,
    ) -> Self {
        let instances = if do_default {
            let count = num_instances as usize;
            (0..count)
                .flat_map(|z| {
                    (0..count).flat_map(move |y| {
                        (0..count).map(move |x| {
                            let position = cgmath::Vector3::new(
                                space_between * (x as f32 - num_instances as f32 / 2.0),
                                space_between * (y as f32 - num_instances as f32 / 2.0),
                                space_between * (z as f32 - num_instances as f32 / 2.0),
                            );

                            let rotation = if position.magnitude() == 0.0 {
                                cgmath::Quaternion::from_angle_y(cgmath::Deg(0.0))
                            } else {
                                cgmath::Quaternion::from_axis_angle(
                                    position.normalize(),
                                    cgmath::Deg(45.0),
                                )
                            };

                            Instance { position, rotation }
                        })
                    })
                })
                .collect()
        } else {
            vec![Instance::default()]
        };

        let capacity = instances.len() * 2;
        let buffer_size = (capacity * mem::size_of::<InstanceRaw>()) as u64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(
            &instance_buffer,
            0,
            bytemuck::cast_slice(&instances.iter().map(|i| i.to_raw()).collect::<Vec<_>>()),
        );

        Self {
            instances,
            instance_buffer,
            capacity,
        }
    }

    pub fn add_instance(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        position: cgmath::Vector3<f32>,
        rotation: cgmath::Quaternion<f32>,
    ) {
        if self.instances.len() >= self.capacity {
            self.capacity *= 2;
            let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: (self.capacity * mem::size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            queue.write_buffer(
                &new_buffer,
                0,
                bytemuck::cast_slice(
                    &self
                        .instances
                        .iter()
                        .map(|i| i.to_raw())
                        .collect::<Vec<_>>(),
                ),
            );

            self.instance_buffer = new_buffer;
        }

        self.instances.push(Instance { position, rotation });
        let offset = self.instances.len() - 1;
        queue.write_buffer(
            &self.instance_buffer,
            (offset * mem::size_of::<InstanceRaw>()) as u64,
            bytemuck::cast_slice(&[Instance { position, rotation }.to_raw()]),
        );
    }
}

// --- Instance Struct ---
#[repr(C)]
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

impl Default for Instance {
    fn default() -> Self {
        Instance {
            position: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(0.0)),
        }
    }
}

// --- InstanceRaw ---
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
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

#[allow(dead_code, unused, unused_variables)]
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
#[allow(dead_code, unused, unused_variables)]
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
