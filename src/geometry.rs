
use std::mem;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use super::geometry;

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
                    format: wgpu::VertexFormat::Float32x3, // Integer was "Sint32x3"
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

#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
// Update GeometryBuffer to include texture coordinate buffer
pub struct Cube {
    pub position: u16,
    pub material: u16,
    pub points: u32,
    pub rotation: u8,
    pub vertices: [geometry::Vertex; 8], // Use `Self` prefix for clarity
    pub indices: [u16; 36],
    pub texture_coords: [geometry::TexCoord; 8], // Use `Self` prefix for clarity
}

impl Cube {
    #[allow(
        dead_code,
        unused,
        redundant_imports,
        unused_results,
        unused_features,
        unused_variables,
        unused_mut,
        dead_code,
        unused_unsafe,
        unused_attributes,
        non_upper_case_globals
    )]
    pub fn default() -> Self {
        // Cube vertices (8 vertices for a cube)
        const vertices: [geometry::Vertex; 8] = [
            // Front face
            geometry::Vertex { position: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
            geometry::Vertex { position: [0.0, 1.0, 0.0], normal: [0.0, 0.0, 1.0] },
            geometry::Vertex { position: [1.0, 1.0, 0.0], normal: [0.0, 0.0, 1.0] },
            geometry::Vertex { position: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
            // Back face
            geometry::Vertex { position: [0.0, 0.0, -1.0], normal: [0.0, 0.0, 1.0] },
            geometry::Vertex { position: [0.0, 1.0, -1.0], normal: [0.0, 0.0, 1.0] },
            geometry::Vertex { position: [1.0, 1.0, -1.0], normal: [0.0, 0.0, 1.0] },
            geometry::Vertex { position: [1.0, 0.0, -1.0], normal: [0.0, 0.0, 1.0] },
        ];

        const indices: [u16; 36] = [
            // Front face (indices 0-3)
            1, 0, 2, // Triangle 1 (top-right)
            2, 0, 3, // Triangle 2 (bottom-right)

            // Back face (indices 4-7)
            4, 5, 6, // Triangle 1 (top-right)
            4, 6, 7, // Triangle 2 (bottom-right)

            // Bottom face (vertices 0, 4, 7, 3)
            0, 4, 7, // Triangle 1 (bottom)
            0, 7, 3, // Triangle 2 (right)

            // Top face (vertices 1, 5, 6, 2)
            5, 1, 6, // Triangle 1 (top)
            6, 1, 2, // Triangle 2 (right)

            // Right face (vertices 2, 6, 7, 3)
            6, 2, 7, // Triangle 1 (top)
            2, 3, 7, // Triangle 2 (bottom)

            // Left face (vertices 0, 4, 5, 1)
            5, 4, 0, // Triangle 1 (left)
            5, 0, 1, // Triangle 2 (top)
        ];

        // Texture coordinates (8 points for a cube)
        const texture_coords: [geometry::TexCoord; 8] = [
            // Front face vertices (indices 0-3)
            geometry::TexCoord { uv: [1.0, 1.0] },
            geometry::TexCoord { uv: [1.0, 0.0] },
            geometry::TexCoord { uv: [0.0, 0.0] },
            geometry::TexCoord { uv: [0.0, 1.0] }, // Vertex 3 (bottom-right)
            // Back face (vertices 4-7) - reuse front's UVs for simplicity
            geometry::TexCoord { uv: [1.0, 1.0] }, // Vertex 4
            geometry::TexCoord { uv: [1.0, 0.0] }, // Vertex 5
            geometry::TexCoord { uv: [0.0, 0.0] }, // Vertex 6
            geometry::TexCoord { uv: [0.0, 1.0] }, // Vertex 7
        ];

        Self {
            position: 0,
            material: 0,
            points: 0,
            rotation: 0,
            vertices,
            indices,
            texture_coords,
        }
    }
}

pub struct CubeBuffer;

impl CubeBuffer {
    pub fn new(
        device: &wgpu::Device,
        cube: &geometry::Cube, // Add `crate` prefix for clarity
    ) -> geometry::GeometryBuffer { // Use `Self` prefix for clarity
        geometry::GeometryBuffer::new( // Use `Self` prefix for clarity
            &device,
            &cube.indices,
            &cube.vertices,
            &cube.texture_coords,
        )
    }
}

#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
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
        indices: &[u16],
        vertices: &[geometry::Vertex], // Use `Self` prefix for clarity
        texture_coords: &[geometry::TexCoord], // Use `Self` prefix for clarity
    ) -> Self {
        let vertex_buffer: wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let texture_coord_buffer: wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Coordinate Buffer"),
            contents: bytemuck::cast_slice(texture_coords),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer: wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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