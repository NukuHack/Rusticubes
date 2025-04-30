use super::cube_tables::{EDGE_TABLE, TRI_TABLE};
use crate::cube::Chunk;
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

use std::mem;

/// Vertex structure with position, normal, and UV coordinates
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    /// Describes the vertex buffer layout for wgpu
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use wgpu::VertexAttribute;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Builder for chunk meshes
pub struct ChunkMeshBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    current_vertex: u32,
}

impl ChunkMeshBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(Chunk::VOLUME), // should be multiple times the chunk volume but most of it is culled so prop this is enough
            indices: Vec::with_capacity(Chunk::VOLUME),
            current_vertex: 0,
        }
    }

    /// Generates marching cubes mesh for this block
    pub fn add_marching_cube(&mut self, points: u32, position: Vec3) {
        // Early exit if no points are set
        if points == 0 {
            return;
        }

        let base_pos = position;
        let mut edge_vertex_cache = [None; 12];
        let mut case_cache = [0u8; 8]; // Cache case indices for each sub-cube

        // Precompute all case indices first
        for i in 0..8 {
            let (x, y, z) = ((i & 1) as u8, ((i >> 1) & 1) as u8, ((i >> 2) & 1) as u8);

            let idx = [
                (x, y, z),
                (x + 1, y, z),
                (x + 1, y, z + 1),
                (x, y, z + 1),
                (x, y + 1, z),
                (x + 1, y + 1, z),
                (x + 1, y + 1, z + 1),
                (x, y + 1, z + 1),
            ];

            let mut case_index = 0u8;
            for (bit, &(x, y, z)) in idx.iter().enumerate() {
                let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
                if (points & (1u32 << bit_pos)) != 0 {
                    case_index |= 1 << bit;
                }
            }
            case_cache[i] = case_index;
        }

        // Process each sub-cube
        for i in 0..8 {
            let case_index = case_cache[i];

            // Skip empty/full sub-cubes
            if case_index == 0 || case_index == 255 {
                continue;
            }

            let edges = EDGE_TABLE[case_index as usize];
            if edges == 0 {
                continue;
            }

            // Calculate sub-cube offset
            let (x, y, z) = (
                (i & 1) as f32 * 0.5,
                ((i >> 1) & 1) as f32 * 0.5,
                ((i >> 2) & 1) as f32 * 0.5,
            );
            let sub_offset = Vec3::new(x, y, z);

            // Calculate and cache edge vertices
            for edge in 0..12 {
                if (edges & (1 << edge)) != 0 && edge_vertex_cache[edge].is_none() {
                    let [a, b] = EDGE_VERTICES[edge];
                    edge_vertex_cache[edge] = Some(a.lerp(b, 0.5));
                }
            }

            // Generate triangles
            let triangles = &TRI_TABLE[case_index as usize];
            let mut i = 0;
            while i < 16 && triangles[i] != -1 {
                let v0 = edge_vertex_cache[triangles[i] as usize].unwrap();
                let v1 = edge_vertex_cache[triangles[i + 1] as usize].unwrap();
                let v2 = edge_vertex_cache[triangles[i + 2] as usize].unwrap();

                let world_vertices = [
                    base_pos + sub_offset + v0,
                    base_pos + sub_offset + v1,
                    base_pos + sub_offset + v2,
                ];

                self.add_triangle(&world_vertices);
                i += 3;
            }

            // Clear the cache for the next sub-cube
            edge_vertex_cache = [None; 12];
        }
    }

    #[inline]
    pub fn add_triangle(&mut self, vertices: &[Vec3; 3]) {
        // Calculate face normal
        let edge1 = vertices[1] - vertices[0];
        let edge2 = vertices[2] - vertices[0];
        let normal = edge1.cross(edge2).normalize();
        let normal_arr = [normal.x, normal.y, normal.z];

        let base = self.current_vertex as u16;
        for vertex in vertices {
            self.vertices.push(Vertex {
                position: [vertex.x, vertex.y, vertex.z],
                normal: normal_arr,
                uv: [0.0, 0.0], // TODO: Proper UV mapping
            });
        }
        self.indices.extend([base, base + 1, base + 2]);
        self.current_vertex += 3;
    }

    /// Adds a rotated cube to the mesh with face culling
    pub fn add_cube(
        &mut self,
        position: Vec3,
        rotation: Quat,
        texture_map: [f32; 2],
        chunk: &Chunk,
    ) {
        let fs: f32 = texture_map[0];
        let fe: f32 = texture_map[1];
        // Pre-calculate the transform matrix once
        let transform = Mat4::from_rotation_translation(rotation, position);

        // Check face visibility first to avoid unnecessary vertex calculations
        let face_visibility = [
            chunk.is_block_cull(position + Vec3::Z), // Front
            chunk.is_block_cull(position - Vec3::Z), // Back
            chunk.is_block_cull(position + Vec3::Y), // Top
            chunk.is_block_cull(position - Vec3::Y), // Bottom
            chunk.is_block_cull(position + Vec3::X), // Right
            chunk.is_block_cull(position - Vec3::X), // Left
        ];

        // Early exit if no faces are visible
        if !face_visibility.iter().any(|&v| v) {
            return;
        }

        // Transform all vertices at once and store them
        let mut transformed_vertices = [[0.0f32; 3]; 8];
        for (i, vertex) in CUBE_VERTICES.iter().enumerate() {
            let pos = transform * Vec3::from(*vertex).extend(1.0);
            transformed_vertices[i] = [pos.x, pos.y, pos.z];
        }

        // In add_cube, instead of adding all vertices at once:
        for (face_idx, &visible) in face_visibility.iter().enumerate() {
            if visible {
                let face = &CUBE_FACES[face_idx];
                let uv = match face_idx {
                    0 => [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Front
                    1 => [[fe, fs], [fs, fs], [fs, fe], [fe, fe]], // Back
                    2 => [[fs, fe], [fe, fe], [fe, fs], [fs, fs]], // Top
                    3 => [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Bottom
                    4 => [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Right
                    5 => [[fe, fs], [fs, fs], [fs, fe], [fe, fe]], // Left
                    _ => [[0.0, 0.0]; 4],
                };

                let base = self.current_vertex as u16;
                for (i, &vertex_idx) in face.iter().enumerate() {
                    // pushing 24 Vertex in case of a fully visible cube
                    let pos = transformed_vertices[vertex_idx as usize];
                    self.vertices.push(Vertex {
                        // each vertex have 8 f32 as storage making it 256 bits - so 6144 bits per cube
                        position: pos,
                        normal: CUBE_NORMALS[vertex_idx as usize],
                        uv: uv[i],
                    });
                }

                self.indices
                    .extend(&[base, base + 1, base + 2, base + 2, base + 3, base]); // 6 u16 int (96 bits) - so 576 for each cube

                self.current_vertex += 4;
            }
        }
    }
}

/// Edge vertices for marching cubes algorithm
const HALF: f32 = 0.5;
const EDGE_VERTICES: [[Vec3; 2]; 12] = [
    [Vec3::ZERO, Vec3::new(HALF, 0.0, 0.0)], // Edge 0
    [Vec3::new(HALF, 0.0, 0.0), Vec3::new(HALF, 0.0, HALF)], // Edge 1
    [Vec3::new(HALF, 0.0, HALF), Vec3::new(0.0, 0.0, HALF)], // Edge 2
    [Vec3::new(0.0, 0.0, HALF), Vec3::ZERO], // Edge 3
    [Vec3::new(0.0, HALF, 0.0), Vec3::new(HALF, HALF, 0.0)], // Edge 4
    [Vec3::new(HALF, HALF, 0.0), Vec3::new(HALF, HALF, HALF)], // Edge 5
    [Vec3::new(HALF, HALF, HALF), Vec3::new(0.0, HALF, HALF)], // Edge 6
    [Vec3::new(0.0, HALF, HALF), Vec3::new(0.0, HALF, 0.0)], // Edge 7
    [Vec3::ZERO, Vec3::new(0.0, HALF, 0.0)], // Edge 8
    [Vec3::new(HALF, 0.0, 0.0), Vec3::new(HALF, HALF, 0.0)], // Edge 9
    [Vec3::new(HALF, 0.0, HALF), Vec3::new(HALF, HALF, HALF)], // Edge 10
    [Vec3::new(0.0, 0.0, HALF), Vec3::new(0.0, HALF, HALF)], // Edge 11
];

// Cube geometry constants
const CUBE_SIZE: f32 = 1.0; // unit sized cube
const INV_SQRT_3: f32 = 0.577_350_269; // 1/sqrt(3) for normalized diagonals

// Cube vertices (8 corners of a unit cube with left-bottom-front at origin)
const CUBE_VERTICES: [[f32; 3]; 8] = [
    [0.0, 0.0, CUBE_SIZE],             // front-bottom-left (origin)
    [CUBE_SIZE, 0.0, CUBE_SIZE],       // front-bottom-right
    [CUBE_SIZE, CUBE_SIZE, CUBE_SIZE], // front-top-right
    [0.0, CUBE_SIZE, CUBE_SIZE],       // front-top-left
    [0.0, 0.0, 0.0],                   // back-bottom-left
    [CUBE_SIZE, 0.0, 0.0],             // back-bottom-right
    [CUBE_SIZE, CUBE_SIZE, 0.0],       // back-top-right
    [0.0, CUBE_SIZE, 0.0],             // back-top-left
];

// Pre-calculated normals for each vertex (8 normals matching the cube vertices)
const CUBE_NORMALS: [[f32; 3]; 8] = [
    [-INV_SQRT_3, -INV_SQRT_3, INV_SQRT_3],  // front-bottom-left
    [INV_SQRT_3, -INV_SQRT_3, INV_SQRT_3],   // front-bottom-right
    [INV_SQRT_3, INV_SQRT_3, INV_SQRT_3],    // front-top-right
    [-INV_SQRT_3, INV_SQRT_3, INV_SQRT_3],   // front-top-left
    [-INV_SQRT_3, -INV_SQRT_3, -INV_SQRT_3], // back-bottom-left
    [INV_SQRT_3, -INV_SQRT_3, -INV_SQRT_3],  // back-bottom-right
    [INV_SQRT_3, INV_SQRT_3, -INV_SQRT_3],   // back-top-right
    [-INV_SQRT_3, INV_SQRT_3, -INV_SQRT_3],  // back-top-left
];

// Each face defined as 4 indices (will be converted to 6 indices/triangles)
pub const CUBE_FACES: [[u16; 4]; 6] = [
    [0, 1, 2, 3], // Front face
    [5, 4, 7, 6], // Back face
    [3, 2, 6, 7], // Top face
    [4, 5, 1, 0], // Bottom face
    [1, 5, 6, 2], // Right face
    [4, 0, 3, 7], // Left face
];

// --- Geometry Buffer (modified for chunk meshes) ---
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
    pub vertex_capacity: usize,
    pub index_capacity: usize,
}

impl GeometryBuffer {
    pub fn new(device: &wgpu::Device, indices: &[u16], vertices: &[Vertex]) -> Self {
        if vertices.is_empty() && indices.is_empty() {
            return Self::empty(device);
        }

        Self {
            vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            }),
            num_indices: indices.len() as u32,
            num_vertices: vertices.len() as u32,
            vertex_capacity: vertices.len(),
            index_capacity: indices.len(),
        }
    }

    pub fn empty(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Empty Vertex Buffer"),
            size: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Empty Index Buffer"),
            size: std::mem::size_of::<u16>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: 0,
            num_vertices: 0,
            vertex_capacity: 0,
            index_capacity: 0,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        indices: &[u16],
        vertices: &[Vertex],
    ) {
        // Helper function to align sizes
        fn align_size(size: usize, alignment: usize) -> usize {
            ((size + alignment - 1) / alignment) * alignment
        }

        // Update vertex buffer if needed
        if vertices.len() > self.vertex_capacity {
            // Create new buffer if capacity is insufficient
            self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.vertex_capacity = vertices.len();
        } else if !vertices.is_empty() {
            // Update existing buffer with proper alignment
            let vertex_slice = bytemuck::cast_slice(vertices);
            let aligned_size = align_size(vertex_slice.len(), wgpu::COPY_BUFFER_ALIGNMENT as usize);
            let mut aligned_data = vertex_slice.to_vec();
            aligned_data.resize(aligned_size, 0); // Pad with zeros if needed
            queue.write_buffer(&self.vertex_buffer, 0, &aligned_data);
        }

        // Update index buffer if needed
        if indices.len() > self.index_capacity {
            // Create new buffer if capacity is insufficient
            self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
            self.index_capacity = indices.len();
        } else if !indices.is_empty() {
            // Update existing buffer with proper alignment
            let index_slice = bytemuck::cast_slice(indices);
            let aligned_size = align_size(index_slice.len(), wgpu::COPY_BUFFER_ALIGNMENT as usize);
            let mut aligned_data = index_slice.to_vec();
            aligned_data.resize(aligned_size, 0); // Pad with zeros if needed
            queue.write_buffer(&self.index_buffer, 0, &aligned_data);
        }

        self.num_indices = indices.len() as u32;
        self.num_vertices = vertices.len() as u32;
    }
}
