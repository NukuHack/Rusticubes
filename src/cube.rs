use wgpu::util::DeviceExt;
use crate::traits::VectorTypeConversion;
use crate::geometry::GeometryBuffer;
use crate::geometry::Vertex;
use crate::geometry::VERTICES;
use crate::geometry::INDICES;
use crate::geometry::EDGE_TABLE;
use crate::geometry::TRI_TABLE;
use cgmath::Zero;
use cgmath::VectorSpace;
use cgmath::InnerSpace;
use cgmath::{Quaternion, Rotation3, Vector3, Deg};
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
            .field("points", &format!("{:027b}", self.points))
            .field("rotation", &format!("{:015b}", self.rotation))
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
    pub fn new_dot() -> Self { // hexadecimal is my life ... here is in binary in case you need it -> 0b10_000_000_000_000
        Self { material: 1, points: 0x_20_00, ..Self::default() }
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

    #[inline]
    pub fn is_marching(&self) -> bool { self.points != 0 }

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

impl Block {
    /// Set a specific point in the 3x3x3 grid (x,y,z in 0..=2)
    #[inline]
    pub fn set_point(&mut self, (x, y, z, value):(u8,u8,u8,bool)) {
        assert!(x < 3 && y < 3 && z < 3, "Coordinates must be in 0..=2");
        let bit_pos = (x as u32) + (y as u32) * 3 + (z as u32) * 9;
        self.points = if value {
            self.points | (1 << bit_pos)
        } else {
            self.points & !(1 << bit_pos)
        };
    }

    /// Get a specific point in the 3x3x3 grid (x,y,z in 0..=2)
    #[inline]
    pub fn get_point(&self, x: u8, y: u8, z: u8) -> bool {
        assert!(x < 3 && y < 3 && z < 3, "Coordinates must be in 0..=2");
        let bit_pos = (x as u32) + (y as u32) * 3 + (z as u32) * 9;
        (self.points & (1 << bit_pos)) != 0
    }

    /// Precomputed corner positions for marching cubes
    const CORNER_POSITIONS: [Vector3<f32>; 8] = [
        Vector3::new(0.0, 0.0, 0.0),  // 0
        Vector3::new(0.5, 0.0, 0.0),  // 1
        Vector3::new(0.5, 0.0, 0.5),  // 2
        Vector3::new(0.0, 0.0, 0.5),  // 3
        Vector3::new(0.0, 0.5, 0.0),  // 4
        Vector3::new(0.5, 0.5, 0.0),  // 5
        Vector3::new(0.5, 0.5, 0.5),  // 6
        Vector3::new(0.0, 0.5, 0.5),  // 7
    ];

    /// Generate mesh for a 3x3x3 marching cubes cell
    pub fn generate_marching_cubes_mesh(&self, position: Vector3<u32>, builder: &mut ChunkMeshBuilder) {
        let base_pos = position.to_vec3_f32() - Vector3::new(0.0, 0.0, 1.0);

        // Process each of the 8 sub-cubes in the 3x3x3 grid
        for sub_z in 0..2 {
            for sub_y in 0..2 {
                for sub_x in 0..2 {
                    // Get the 8 corner points for this sub-cube
                    let mut case_index = 0;
                    case_index |= (self.get_point(sub_x, sub_y, sub_z) as u8) << 0;
                    case_index |= (self.get_point(sub_x + 1, sub_y, sub_z) as u8) << 1;
                    case_index |= (self.get_point(sub_x + 1, sub_y, sub_z + 1) as u8) << 2;
                    case_index |= (self.get_point(sub_x, sub_y, sub_z + 1) as u8) << 3;
                    case_index |= (self.get_point(sub_x, sub_y + 1, sub_z) as u8) << 4;
                    case_index |= (self.get_point(sub_x + 1, sub_y + 1, sub_z) as u8) << 5;
                    case_index |= (self.get_point(sub_x + 1, sub_y + 1, sub_z + 1) as u8) << 6;
                    case_index |= (self.get_point(sub_x, sub_y + 1, sub_z + 1) as u8) << 7;

                    // Skip empty or full sub-cubes
                    if case_index == 0 || case_index == 255 { continue; }

                    // Get edges for this case
                    let edges = EDGE_TABLE[case_index as usize];
                    if edges == 0 { continue; }

                    // Calculate vertex positions for each edge that is crossed
                    let mut edge_vertices = [Vector3::zero(); 12];
                    for edge in 0..12 {
                        if (edges & (1 << edge)) != 0 {
                            let (a, b) = match edge {
                                0 => (0, 1),
                                1 => (1, 2),
                                2 => (2, 3),
                                3 => (3, 0),
                                4 => (4, 5),
                                5 => (5, 6),
                                6 => (6, 7),
                                7 => (7, 4),
                                8 => (0, 4),
                                9 => (1, 5),
                                10 => (2, 6),
                                11 => (3, 7),
                                _ => unreachable!(),
                            };
                            
                            // Linear interpolation at midpoint
                            edge_vertices[edge] = Self::CORNER_POSITIONS[a].lerp(Self::CORNER_POSITIONS[b], 0.5);
                        }
                    }

                    // Generate triangles from the triangle table
                    let triangles = &TRI_TABLE[case_index as usize];
                    let sub_offset = Vector3::new(
                        sub_x as f32 * 0.5,
                        sub_y as f32 * 0.5,
                        sub_z as f32 * 0.5
                    );

                    for i in (0..16).step_by(3) {
                        if triangles[i] == -1 { break; }
                        
                        let indices = [triangles[i] as usize, triangles[i+1] as usize, triangles[i+2] as usize];
                        let vertices = indices.map(|idx| base_pos + sub_offset + edge_vertices[idx]);
                        
                        builder.add_triangle(&vertices);
                    }
                }
            }
        }
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

// New u64-based chunk coordinate representation
// Format: [X:26 (signed), Y:12 (signed), Z:26 (signed)]
pub type ChunkCoord = u64;

#[allow(dead_code, unused)]
pub trait ChunkCoordHelp {
    const X_MASK: u64 = 0x03FFFFFF; // 26 bits
    const X_SHIFT: u32 = 38;
    const Y_MASK: u64 = 0x0FFF;     // 12 bits
    const Y_SHIFT: u32 = 26;
    const Z_MASK: u64 = 0x03FFFFFF; // 26 bits
    const Z_SHIFT: u32 = 0;

    fn new(x: i32, y: i32, z: i32) -> Self;

    fn pack(x: i32, y: i32, z: i32) -> Self;

    fn unpack(coord: &Self) -> (i32, i32, i32);

    fn extract_x(coord: &Self) -> i32;

    fn extract_y(coord: &Self) -> i32;

    fn extract_z(coord: &Self) -> i32;

    fn to_world_pos(&self) -> Vector3<i32>;

    fn from_world_pos(world_pos: Vector3<i32>) -> Self;
}

impl ChunkCoordHelp for ChunkCoord {
    #[inline]
    fn new(x: i32, y: i32, z: i32) -> Self {
        Self::pack(x, y, z)
    }

    #[inline]
    fn pack(x: i32, y: i32, z: i32) -> Self {
        debug_assert!(x >= -(1 << 25) && x < (1 << 25), "X coordinate out of range");
        debug_assert!(y >= -(1 << 11) && y < (1 << 11), "Y coordinate out of range");
        debug_assert!(z >= -(1 << 25) && z < (1 << 25), "Z coordinate out of range");
        
        ((x as u64 & Self::X_MASK) << Self::X_SHIFT) |
        ((y as u64 & Self::Y_MASK) << Self::Y_SHIFT) |
        (z as u64 & Self::Z_MASK)
    }

    #[inline]
    fn unpack(coord: &Self) -> (i32, i32, i32) {
        (
            Self::extract_x(coord),
            Self::extract_y(coord),
            Self::extract_z(coord),
        )
    }

    #[inline]
    fn extract_x(coord: &Self) -> i32 {
        // Sign extend the 26-bit value
        (((*coord >> Self::X_SHIFT) as i32) << 6) >> 6
    }

    #[inline]
    fn extract_y(coord: &Self) -> i32 {
        // Sign extend the 12-bit value
        (((*coord >> Self::Y_SHIFT) as i32 & Self::Y_MASK as i32) << 20) >> 20
    }

    #[inline]
    fn extract_z(coord: &Self) -> i32 {
        // Sign extend the 26-bit value
        ((*coord as i32 & Self::Z_MASK as i32) << 6) >> 6
    }

    #[inline]
    fn to_world_pos(&self) -> Vector3<i32> {
        let (x, y, z) = Self::unpack(self);
        Vector3::new(
            x * Chunk::CHUNK_SIZE_I,
            y * Chunk::CHUNK_SIZE_I,
            z * Chunk::CHUNK_SIZE_I,
        )
    }

    #[inline]
    fn from_world_pos(world_pos: Vector3<i32>) -> Self {
        let chunk_size = Chunk::CHUNK_SIZE_I;
        Self::new(
            world_pos.x.div_euclid(chunk_size),
            world_pos.y.div_euclid(chunk_size),
            world_pos.z.div_euclid(chunk_size),
        )
    }
}


#[derive(Clone, Debug)]
pub struct Chunk {
    pub blocks: FastMap<u16, Block>,  // Key is packed position (x,y,z)
    pub dirty: bool,  // For mesh regeneration
    pub mesh: Option<super::geometry::GeometryBuffer>,
    bind_group: Option<wgpu::BindGroup>,
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
            bind_group: None,
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
        let pos = Self::local_to_block_pos(local_pos);
        self.blocks.get(&pos)
    }

    pub fn get_block_mut(&mut self, local_pos: Vector3<u32>) -> Option<&mut Block> {
        let pos = Self::local_to_block_pos(local_pos);
        self.blocks.get_mut(&pos)
    }

    pub fn set_block(&mut self, local_pos: Vector3<u32>, block: Block) {
        let pos = Self::local_to_block_pos(local_pos);
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
    pub fn local_to_block_pos(local_pos: Vector3<u32>) -> u16 {
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

    pub fn make_mesh(&mut self, device: &wgpu::Device, force: bool) {
        if !force || (!self.dirty && self.mesh.is_some()) {
            return;
        }

        let mut builder = ChunkMeshBuilder::new();
        
        for (&pos, block) in &self.blocks {
            if block.is_empty() {
                continue;
            }
            
            let local_pos = Self::position_to_local(pos);
            if block.is_marching() {
                block.generate_marching_cubes_mesh(local_pos, &mut builder);
                continue;
            }

            builder.add_cube(local_pos.to_vec3_f32(), block.rotation_to_quaternion());
        }

        self.mesh = Some(builder.build(device));
        self.dirty = false;
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
                let pos = Chunk::local_to_block_pos(local);
                chunk.blocks.get(&pos)
            })
    }

    pub fn get_block_mut(&mut self, world_pos: Vector3<i32>) -> Option<&mut Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get_mut(&chunk_coord)
            .and_then(|chunk| {
                let local = Chunk::world_to_local_pos(world_pos);
                let pos = Chunk::local_to_block_pos(local);
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
        unsafe {
            let state = super::get_state();
            let device = &state.render_context.device;
            let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

            // Unload existing chunk if present
            if self.get_chunk(chunk_coord).is_some() {
                super::get_state().data_system.world.unload_chunk(chunk_coord);
            }

            // Attempt to load chunk
            let chunk = match Chunk::load() {
                Some(c) => c,
                None => return false,
            };
            
            // Create position buffer (only storing Vector3<f32> now)
            let world_pos = chunk_coord.to_world_pos().to_vec3_f32();
            let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Position Buffer"),
                contents: bytemuck::cast_slice(&[world_pos.x, world_pos.y, world_pos.z, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            
            // Create bind group
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &chunk_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: position_buffer.as_entire_binding(),
                    },
                ],
                label: Some("chunk_bind_group"),
            });
            
            // Update chunk with new GPU resources
            let updated_chunk = Chunk {
                bind_group: Some(bind_group),
                ..chunk
            };
            
            self.set_chunk(chunk_coord, updated_chunk);
            true
        }
    }

    pub fn update_loaded_chunks(&mut self, center: Vector3<i32>, radius: u32) {
        let chunk_pos = ChunkCoord::from_world_pos(center);
        let (center_x, center_y, center_z) = ChunkCoord::unpack(&chunk_pos);
        let radius_i32 = radius as i32;
        let radius_sq = (radius * radius) as i32;
        let partially_unload = true;

        if partially_unload {
            // Track chunks we want to unload
            let mut chunks_to_unload = Vec::new();

            // Use loaded_chunks for faster iteration
            for &coord in &self.loaded_chunks {
                let (x, y, z) = ChunkCoord::unpack(&coord);
                let dx = x - center_x;
                let dy = y - center_y;
                let dz = z - center_z;
                
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
            let min_x = center_x - radius_i32;
            let max_x = center_x + radius_i32;
            let min_y = center_y - radius_i32;
            let max_y = center_y + radius_i32;
            let min_z = center_z - radius_i32;
            let max_z = center_z + radius_i32;

            // Unload distant chunks
            self.loaded_chunks.retain(|&coord| {
                let (x, y, z) = ChunkCoord::unpack(&coord);
                let dx = x - center_x;
                let dy = y - center_y;
                let dz = z - center_z;
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
                        let dx = x - center_x;
                        let dy = y - center_y;
                        let dz = z - center_z;
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
        for (&_coord, chunk) in self.chunks.iter_mut() {
            chunk.make_mesh(device, false);
        }
    }

    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for (&_coord, chunk) in self.chunks.iter() {
            if let Some(mesh) = &chunk.mesh {
                if let Some(bind_group) = &chunk.bind_group {
                    render_pass.set_bind_group(2, bind_group, &[]);
                    
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                }
            }
        }
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

    pub fn add_cube(&mut self, position: Vector3<f32>, rotation: cgmath::Quaternion<f32>) {
        // Transform matrix
        let transform =
            cgmath::Matrix4::from_translation(position) * cgmath::Matrix4::from(rotation);
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
        }
        self.current_vertex += VERTICES.len() as u32;

        // Add indices with offset
        self.indices.extend(INDICES.iter().map(|&i| start_vertex as u16 + i));
    }
    #[inline]
    pub fn add_face(
        &mut self,
        position: Vector3<f32>,
        rotation: Quaternion<f32>,
        corners: [Vector3<f32>; 4],
        normal: Vector3<f32>,
    ) {
        let normal = rotation * normal;
        let base = self.vertices.len() as u16;

        for corner in corners {
            let pos = position + rotation * corner;
            self.vertices.push(Vertex {
                position: pos.into(),
                normal: normal.into(),
                uv: [0.5; 2], // Simplified UVs
            });
        }

        self.indices.extend(&[base, base+1, base+2, base, base+2, base+3]);
    }

    // New function to add individual vertices for marching cubes
    #[inline]
    pub fn add_vertex(&mut self, position: Vector3<f32>, normal: Vector3<f32>) {
        self.vertices.push(Vertex {
            position: position.into(),
            normal: normal.into(),
            uv: [0.0, 0.0], // Default UVs, could be improved
        });
        self.indices.push(self.current_vertex as u16);
        self.current_vertex += 1;
    }
    #[inline]
    pub fn add_triangle(&mut self, vertices: &[Vector3<f32>; 3]) {
        let normal = (vertices[1] - vertices[0])
            .cross(vertices[2] - vertices[0])
            .normalize();

        let base = self.vertices.len() as u16;
        for vertex in vertices {
            self.vertices.push(Vertex {
                position: (*vertex).into(),
                normal: normal.into(),
                uv: [0.0; 2],
            });
        }
        self.indices.extend(&[base, base+1, base+2]);
    }

    #[inline]
    pub fn build(self, device: &wgpu::Device) -> GeometryBuffer {
        GeometryBuffer::new(device, &self.indices, &self.vertices)
    }
}