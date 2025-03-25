
use wgpu::{
    Buffer,
    util::DeviceExt,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    #[allow(unused)]
    position: [f32; 3],
    tex_coords: [f32; 2],
}
impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2, // NEW!
                },
            ]
        }
    }
}

pub const VERTICES: &[self::Vertex] = &[
    self::Vertex {
        position: [-0.0625, 0.5, 0.0],
        tex_coords: [0.4375, 0.0],
    }, // A
    self::Vertex {
        position: [-0.5, 0.0625, 0.0],
        tex_coords: [0.0, 0.4375],
    }, // B
    self::Vertex {
        position: [-0.25, -0.4375, 0.0],
        tex_coords: [0.25, 0.9375],
    }, // C
    self::Vertex {
        position: [0.375, -0.375, 0.0],
        tex_coords: [0.875, 0.84375],
    }, // D
    self::Vertex {
        position: [0.4375, 0.25, 0.0],
        tex_coords: [0.9375, 0.25],
    }, // E
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
    pub num_vertices:u32,
}

impl GeometryBuffer {
    pub fn new(
        device: &wgpu::Device,
        indices: &[u16],
        vertices: &[self::Vertex],
    ) -> Self {

        let vertex_buffer: Buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer: Buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_indices: u32 = indices.len() as u32;
        let num_vertices: u32 = vertices.len() as u32;

        Self {
            vertex_buffer,
            index_buffer,
            num_indices,
            num_vertices,
        }
    }
}