
use cgmath::{Quaternion, Matrix4, Rotation3, Vector3, Deg};
use super::geometry::Vertex;
use std::collections::{HashMap, HashSet};
use ahash::AHasher;
use std::hash::BuildHasherDefault;

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Stores rotations for X, Y, Z as 5-bit fields: [X:5, Y:5, Z:5, Empty:1]
/// Stores 3x3x3 points as a 32-bit "array" [Points: 27, Empty: 5]
#[derive(Clone, Copy)]
pub struct Block {
    pub material: u16,    // Material info (unused in current implementation)
    pub points: u32,      // 3x3x3 points (27 bits used)
    pub rotation: u16,    // [X:5, Y:5, Z:5, Empty:1]
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field("material", &format_args!("{:?}", self.material))
            .field("points", &format_args!("{:?}", self.points))
            .field("rotation", &format_args!("{:?}", self.rotation))
            .finish()
    }
}

impl Block {
    pub const ROT_MASK_X: u16 = 0b11111;
    pub const ROT_SHIFT_X: u32 = 0;
    pub const ROT_MASK_Y: u16 = 0b11111 << 5;
    pub const ROT_SHIFT_Y: u32 = 5;
    pub const ROT_MASK_Z: u16 = 0b11111 << 10;
    pub const ROT_SHIFT_Z: u32 = 10;
    
    #[inline]
    pub fn default() -> Self {
        Self { material: 0, rotation: 0, points: 0}
    }

    #[inline]
    pub fn new() -> Self {
        Self { material: 1, ..Self::default() }
    }

    #[inline]
    pub fn new_rot(rotation: u16) -> Self {
        Self {
            material: 1,
            rotation,
            ..Self::default()
        }
    }
    
    pub fn new_rot_raw(rotation: Quaternion<f32>) -> Self {
        Self {
            material: 1,
            rotation: quaternion_to_rotation(rotation),
            ..Self::default()
        }
    }

    /// Extract individual rotation components (0-31)
    #[inline]
    pub fn get_x_rotation(&self) -> u16 { (self.rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X }
    #[inline]
    pub fn get_y_rotation(&self) -> u16 { (self.rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y }
    #[inline]
    pub fn get_z_rotation(&self) -> u16 { (self.rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z }

    /// Rotation snapping and conversion to quaternion
    pub fn rotation_to_quaternion(&self) -> Quaternion<f32> {
        let angles = [self.get_x_rotation(), self.get_y_rotation(), self.get_z_rotation()]
            .map(|r| Deg(r as f32 * (360.0 / 32.0)));
        
        Quaternion::from_angle_z(angles[2]) *
        Quaternion::from_angle_y(angles[1]) *
        Quaternion::from_angle_x(angles[0])
    }

    #[inline]
    pub fn is_empty(&self) -> bool { self.material == 0 }

    pub fn rotate(&mut self, axis: char, steps: u16) {
        let (current, mask, shift) = match axis {
            'x' => (self.get_x_rotation(), Self::ROT_MASK_X, Self::ROT_SHIFT_X),
            'y' => (self.get_y_rotation(), Self::ROT_MASK_Y, Self::ROT_SHIFT_Y),
            'z' => (self.get_z_rotation(), Self::ROT_MASK_Z, Self::ROT_SHIFT_Z),
            _ => unreachable!(),
        };
        
        let new_rot = (current + steps) % 32;
        self.rotation = (self.rotation & !mask) | (new_rot << shift);
    }
    
    pub fn set_rotation(&mut self, x: u16, y: u16, z: u16) {
        self.rotation = (x & 0x1F) 
            | ((y & 0x1F) << 5) 
            | ((z & 0x1F) << 10);
    }
}

/// Convert a quaternion to the packed u16 rotation format
pub fn quaternion_to_rotation(rotation: Quaternion<f32>) -> u16 {
    let angles = [
        (2.0 * (rotation.s * rotation.v.x + rotation.v.y * rotation.v.z)).atan2(1.0 - 2.0 * (rotation.v.x.powi(2) + rotation.v.y.powi(2))),
        (2.0 * (rotation.s * rotation.v.y - rotation.v.z * rotation.v.x)).asin(),
        (2.0 * (rotation.s * rotation.v.z + rotation.v.x * rotation.v.y)).atan2(1.0 - 2.0 * (rotation.v.y.powi(2) + rotation.v.z.powi(2)))
    ];

    const SCALE: f32 = 31.0 / (2.0 * std::f32::consts::PI);
    let bits: [u16; 3] = angles.map(|a| ((a.rem_euclid(2.0 * std::f32::consts::PI) * SCALE).round() as u16 & 0x1F));
    
    bits[0] | (bits[1] << 5) | (bits[2] << 10)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn to_world_pos(&self) -> Vector3<i32> {
        Vector3::new(
            self.x * Chunk::CHUNK_SIZE_I,
            self.y * Chunk::CHUNK_SIZE_I,
            self.z * Chunk::CHUNK_SIZE_I,
        )
    }

    #[inline]
    pub fn from_world_pos(world_pos: Vector3<i32>) -> Self {
        Self {
            x: world_pos.x.div_euclid(Chunk::CHUNK_SIZE_I),
            y: world_pos.y.div_euclid(Chunk::CHUNK_SIZE_I),
            z: world_pos.z.div_euclid(Chunk::CHUNK_SIZE_I),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub blocks: FastMap<u16, Block>,  // Key is packed position (x,y,z)
    pub dirty: bool,  // For mesh regeneration
    pub mesh: Option<super::geometry::GeometryBuffer>,
}

impl Chunk {
    pub const CHUNK_SIZE: usize = 16;
    pub const CHUNK_SIZE_I: i32 = Self::CHUNK_SIZE as i32;

    /// Creates a new empty chunk
    pub fn empty() -> Self {
        Self {
            blocks: FastMap::with_capacity_and_hasher(
                Self::CHUNK_SIZE.pow(3),
                BuildHasherDefault::<AHasher>::default(),
            ),
            dirty: false,
            mesh: None,
        }
    }

    /// Creates a new filled chunk
    pub fn new() -> Self {
        let mut chunk = Self::empty();
        let capacity = Self::CHUNK_SIZE.pow(3);
        chunk.blocks.reserve(capacity);
        
        for x in 0..Self::CHUNK_SIZE {
            for y in 0..Self::CHUNK_SIZE {
                for z in 0..Self::CHUNK_SIZE {
                    let pos = ((x as u16) << 8) | ((y as u16) << 4) | z as u16;
                    chunk.blocks.insert(pos, Block::new());
                }
            }
        }
        
        chunk.dirty = true;
        chunk
    }

    #[inline]
    pub fn load() -> Option<Self> {
        Some(Self::new())
    }

    pub fn get_block(&self, local_pos: Vector3<u32>) -> Option<&Block> {
        let pos = Self::local_to_position(local_pos);
        self.blocks.get(&pos)
    }

    pub fn get_block_mut(&mut self, local_pos: Vector3<u32>) -> Option<&mut Block> {
        let pos = Self::local_to_position(local_pos);
        self.blocks.get_mut(&pos)
    }

    pub fn set_block(&mut self, local_pos: Vector3<u32>, block: Block) {
        let pos = Self::local_to_position(local_pos);
        if block.is_empty() {
            self.blocks.remove(&pos);
        } else {
            self.blocks.insert(pos, block);
        }
        self.dirty = true;
    }

    /// Convert world position to local chunk coordinates
    #[inline]
    pub fn world_to_local_pos(world_pos: Vector3<i32>) -> Vector3<u32> {
        Vector3::new(
            world_pos.x.rem_euclid(Self::CHUNK_SIZE_I) as u32,
            world_pos.y.rem_euclid(Self::CHUNK_SIZE_I) as u32,
            world_pos.z.rem_euclid(Self::CHUNK_SIZE_I) as u32,
        )
    }

    /// Convert local coordinates to packed position key
    #[inline]
    pub fn local_to_position(local_pos: Vector3<u32>) -> u16 {
        ((local_pos.x as u16) << 8) | ((local_pos.y as u16) << 4) | (local_pos.z as u16)
    }

    /// Convert packed position key to local coordinates
    #[inline]
    pub fn position_to_local(pos: u16) -> Vector3<u32> {
        Vector3::new(
            ((pos >> 8) & 0xF) as u32,
            ((pos >> 4) & 0xF) as u32,
            (pos & 0xF) as u32,
        )
    }

    pub fn make_mesh(&mut self, device: &wgpu::Device, chunk_coord: ChunkCoord, force: bool) {
        if !force && self.mesh.is_some() {
            return; // Skip if no changes
        }

        let mut builder = ChunkMeshBuilder::new();
        
        for (&pos, block) in &self.blocks {
            if block.is_empty() {
                continue;
            }

            let local_pos = Self::position_to_local(pos);
            // Calculate world position by adding chunk offset
            let world_pos_f32 = Vector3::new(
                (chunk_coord.x * Self::CHUNK_SIZE_I as i32 + local_pos.x as i32) as f32,
                (chunk_coord.y * Self::CHUNK_SIZE_I as i32 + local_pos.y as i32) as f32,
                (chunk_coord.z * Self::CHUNK_SIZE_I as i32 + local_pos.z as i32) as f32
            );

            builder.add_cube(world_pos_f32, block.rotation_to_quaternion());
        }

        self.mesh = Some(builder.build(device));
        self.dirty = false;
    }
}

// --- Chunk Mesh Builder ---
pub struct ChunkMeshBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    current_vertex: u32,
}

impl ChunkMeshBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(4096 * 8),
            indices: Vec::with_capacity(4096 * 36),
            current_vertex: 0,
        }
    }

    pub fn add_cube(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>) {
        // Transform matrix
        let transform = Matrix4::from_translation(position) * Matrix4::from(rotation);
        let start_vertex = self.current_vertex;

        // Add transformed vertices
        for vertex in &VERTICES {
            let pos = transform * Vector3::from(vertex.position).extend(1.0);
            let normal = rotation * Vector3::from(vertex.normal);
            
            self.vertices.push(Vertex {
                position: pos.truncate().into(),
                normal: normal.into(),
                uv: vertex.uv,
            });
            self.current_vertex += 1;
        }

        for index in INDICES {
            self.indices.push(start_vertex as u16 + index);
        }
    }

    #[inline]
    pub fn build(self, device: &wgpu::Device) -> super::geometry::GeometryBuffer {
        super::geometry::GeometryBuffer::new(device, &self.indices, &self.vertices)
    }
}

pub struct BlockBuffer;

impl BlockBuffer {
    #[inline]
    pub fn new(device: &wgpu::Device) -> super::geometry::GeometryBuffer {
        super::geometry::GeometryBuffer::new(device, &INDICES, &VERTICES)
    }
}


#[derive(Debug, Clone)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
}

impl World {
    /// Create an empty world with no chunks
    #[inline]
    pub fn empty() -> Self {
        Self {
            chunks: FastMap::with_capacity_and_hasher(
                10000,
                BuildHasherDefault::<AHasher>::default(),
            ),
            loaded_chunks: HashSet::with_capacity(10000),
        }
    }

    #[inline]
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    #[inline]
    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
        self.chunks.get_mut(&coord)
    }

    pub fn get_block(&self, world_pos: Vector3<i32>) -> Option<&Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get(&chunk_coord)
            .and_then(|chunk| {
                let local = Chunk::world_to_local_pos(world_pos);
                let pos = Chunk::local_to_position(local);
                chunk.blocks.get(&pos)
            })
    }

    pub fn get_block_mut(&mut self, world_pos: Vector3<i32>) -> Option<&mut Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get_mut(&chunk_coord)
            .and_then(|chunk| {
                let local = Chunk::world_to_local_pos(world_pos);
                let pos = Chunk::local_to_position(local);
                chunk.blocks.get_mut(&pos)
            })
    }

    pub fn set_block(&mut self, world_pos: Vector3<i32>, block: Block) {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        
        if !self.chunks.contains_key(&chunk_coord) {
            self.set_chunk(chunk_coord, Chunk::empty());
        }
        
        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            let local = Chunk::world_to_local_pos(world_pos);
            chunk.set_block(local, block);
        }
    }

    #[inline]
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoord) -> bool {
        let chunk: Option<Chunk> = Chunk::load();
        if let Some(chunk) = chunk {
            self.set_chunk(chunk_coord, chunk);
            true
        } else {
            false
        }
    }

    pub fn update_loaded_chunks(&mut self, center: Vector3<i32>, radius: u32) {
        let chunk_pos = ChunkCoord::from_world_pos(center);
        let radius_i32 = radius as i32;
        let radius_sq = (radius * radius) as i32;
        let partially_unload = true; // partially unload will be faster if there are less chunks to unload
        // if it is off that will be faster if there are less chunks to keep - more chunks to unload

        if partially_unload {
            // Track chunks we want to keep
            let mut chunks_to_unload = Vec::new();

            // Pre-compute center components
            let center_x = chunk_pos.x;
            let center_y = chunk_pos.y;
            let center_z = chunk_pos.z;

            // Use loaded_chunks for faster iteration
            for &coord in &self.loaded_chunks {
                let dx = coord.x - center_x;
                let dy = coord.y - center_y;
                let dz = coord.z - center_z;
                
                if dx * dx + dy * dy + dz * dz > radius_sq {
                    chunks_to_unload.push(coord);
                }
            }

            // Unload chunks
            for coord in chunks_to_unload {
                self.chunks.remove(&coord);
                self.loaded_chunks.remove(&coord);
            }

            // Load new chunks in range
            for dx in -radius_i32..=radius_i32 {
                for dy in -radius_i32..=radius_i32 {
                    for dz in -radius_i32..=radius_i32 {
                        if dx * dx + dy * dy + dz * dz > radius_sq {
                            continue;
                        }
                        let x = center_x + dx;
                        let y = center_y + dy;
                        let z = center_z + dz;
                        let chunk_coord = ChunkCoord::new(x, y, z);
                        if !self.loaded_chunks.contains(&chunk_coord) {
                            self.load_chunk(chunk_coord);
                        }
                    }
                }
            }
        } else {
            // Calculate bounds once
            let min_x = chunk_pos.x - radius_i32;
            let max_x = chunk_pos.x + radius_i32;
            let min_y = chunk_pos.y - radius_i32;
            let max_y = chunk_pos.y + radius_i32;
            let min_z = chunk_pos.z - radius_i32;
            let max_z = chunk_pos.z + radius_i32;

            // Unload distant chunks
            self.loaded_chunks.retain(|&coord| {
                let dx = coord.x - chunk_pos.x;
                let dy = coord.y - chunk_pos.y;
                let dz = coord.z - chunk_pos.z;
                let keep = dx * dx + dy * dy + dz * dz <= radius_sq;
                if !keep {
                    self.chunks.remove(&coord);
                }
                keep
            });

            // Load new chunks
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    for z in min_z..=max_z {
                        let dx = x - chunk_pos.x;
                        let dy = y - chunk_pos.y;
                        let dz = z - chunk_pos.z;
                        if dx * dx + dy * dy + dz * dz > radius_sq {
                            continue;
                        }
                        let coord = ChunkCoord::new(x, y, z);
                        if !self.loaded_chunks.contains(&coord) {
                            self.load_chunk(coord);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn set_chunk(&mut self, chunk_coord: ChunkCoord, chunk: Chunk) {    
        self.chunks.insert(chunk_coord, chunk);
        self.loaded_chunks.insert(chunk_coord);
    }

    #[inline]
    pub fn unload_chunk(&mut self, chunk_coord: ChunkCoord) {
        self.chunks.remove(&chunk_coord);
        self.loaded_chunks.remove(&chunk_coord);
    }

    pub fn make_chunk_meshes(&mut self, device: &wgpu::Device) {
        for (coord, chunk) in self.chunks.iter_mut() {
            if chunk.dirty {
                chunk.make_mesh(device, *coord, false);
            }
        }
    }

    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let mut chunks: Vec<_> = self.chunks.values().collect();
        chunks.sort_by(|a, b| {
            // Implement your sorting logic based on camera position
            std::cmp::Ordering::Equal
        });

        for chunk in chunks {
            if let Some(mesh) = &chunk.mesh {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }
}

const VERTICES: [Vertex; 8] = [
    Vertex { position: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
    Vertex { position: [0.0, 1.0, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
    Vertex { position: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
    Vertex { position: [0.0, 0.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
    Vertex { position: [0.0, 1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
    Vertex { position: [1.0, 0.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
];

const INDICES: [u16; 36] = [
    1, 0, 2, 3, 2, 0, // Front face (z=0)
    4, 5, 6, 6, 7, 4, // Back face (z=-1)
    0, 4, 7, 3, 0, 7, // Bottom (y=0)
    5, 1, 6, 1, 2, 6, // Top (y=1)
    6, 2, 7, 2, 3, 7, // Right (x=1)
    4, 0, 5, 0, 1, 5, // Left (x=0)
];