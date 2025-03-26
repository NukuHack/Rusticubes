use std::mem;
use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
    pub position: [i32; 3], // Changed to integer positions
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
                    format: wgpu::VertexFormat::Sint32x3, // Integer position format
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[i32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    // front
    Vertex { position: [0, 0, -1], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [0, 1, -1], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [1, 1, -1], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [1, 0, -1], normal: [0.0, 0.0, 1.0] },
    // back
    Vertex { position: [0, 0, -2], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [0, 1, -2], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [1, 1, -2], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [1, 0, -2], normal: [0.0, 0.0, 1.0] },
];

pub const INDICES: &[u16] = &[
    // front
    1, 0, 3, 2, 1, 3,
    // back
    4, 5, 7, 5, 6, 7,
    // Left side face
    0, 4, 7, 0, 7, 3,
    // TOP
    5, 1, 6, 6, 1, 2,
    // Right
    0, 1, 5, 0, 5, 4,
    // Bottom face
    2, 3, 7, 2, 7, 6,
];

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
                shader_location: 2, // Matches @location(2) in shader
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

// Update GeometryBuffer to include texture coordinate buffer
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub texture_coord_buffer: wgpu::Buffer, // New field
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
}

impl GeometryBuffer {
    pub fn new(
        device: &wgpu::Device,
        indices: &[u16],
        vertices: &[Vertex],
        texture_coords: &[TexCoord], // New parameter
    ) -> Self {
        let vertex_buffer:wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let texture_coord_buffer:wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Coordinate Buffer"),
            contents: bytemuck::cast_slice(texture_coords),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer:wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            texture_coord_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            num_vertices: vertices.len() as u32,
        }
    }
}

// Define texture coordinates separately
pub const TEXTURE_COORDS: &[TexCoord] = &[
    TexCoord { uv: [1.0, 1.0] },
    TexCoord { uv: [1.0, 0.0] },
    TexCoord { uv: [0.0, 0.0] },
    TexCoord { uv: [0.0, 1.0] },
];