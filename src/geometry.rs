use std::mem;
use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, Buffer};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3], // Added normal vector
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress, // Updated offset calculation
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0625, 0.5, 0.0], tex_coords: [0.4375, 0.0], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [-0.5, 0.0625, 0.0], tex_coords: [0.0, 0.4375], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [-0.25, -0.4375, 0.0], tex_coords: [0.25, 0.9375], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [0.375, -0.375, 0.0], tex_coords: [0.875, 0.84375], normal: [0.0, 0.0, 1.0] },
    Vertex { position: [0.4375, 0.25, 0.0], tex_coords: [0.9375, 0.25], normal: [0.0, 0.0, 1.0] },
];

pub const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

pub struct GeometryBuffer {
    #[allow(unused)]
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
}

impl GeometryBuffer {
    pub fn new(
        device: &wgpu::Device,
        indices: &[u16],
        vertices: &[Vertex],
    ) -> Self {
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
}