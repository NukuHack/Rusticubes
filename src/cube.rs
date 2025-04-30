use super::cube_tables::{EDGE_TABLE, TRI_TABLE};
use super::geometry::Vertex;
use ahash::AHasher;
use glam::{Mat4, Quat, Vec3};
use std::{
    collections::{HashMap, HashSet},
    f32::consts::{PI, TAU},
    hash::BuildHasherDefault,
};
use wgpu::util::DeviceExt;

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Block {
    None = 0,
    Simple(u16, u8),  // material, packed rotation
    Marching(u16, u32), // material, density field (27 bits in 4 bytes)
}

impl Block {
    // Rotation bit masks and shifts
    const ROT_MASK_X: u8 = 0b0000_0011;
    const ROT_MASK_Y: u8 = 0b0000_1100;
    const ROT_MASK_Z: u8 = 0b0011_0000;
    const ROT_SHIFT_X: u8 = 0;
    const ROT_SHIFT_Y: u8 = 2;
    const ROT_SHIFT_Z: u8 = 4;

    /// Creates a default empty block
    #[inline]
    pub const fn default() -> Self {
        Self::Simple(0,0)
    }

    /// Creates a new simple block with default material
    #[inline]
    pub const fn new() -> Self {
        Self::Simple(1,0)
    }

    /// Creates a new simple block with default material
    #[inline]
    pub const fn new_conf(material: u16, rotation: u8) -> Self {
        Self::Simple(material,rotation)
    }

    /// Creates a new marching cubes block with center point set
    #[inline]
    pub const fn new_dot() -> Self {
        Self::Marching(1,0x20_00)
    }

    /// Creates a new marching cubes block with no point set
    #[inline]
    pub const fn new_march(material: u16, points: u32) -> Self {
        Self::Marching(material,points)
    }

    /// Creates a new block with specified rotation
    #[inline]
    pub const fn new_rot(rotation: u8) -> Self {
        Self::Simple(1,rotation)
    }

    /// Creates a block from a quaternion rotation
    #[inline]
    pub fn new_quat(rotation: Quat) -> Self {
        Self::Simple(1,Self::quat_to_rotation(rotation))
    }

    /// Extracts rotation components (0-3)
    #[inline]
    pub fn get_rotation(&self) -> Option<(u8, u8, u8)> {
        match self {
            Block::Simple(_,rot) => Some((
                (rot & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X,
                (rot & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y,
                (rot & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z,
            )),
            _ => None,
        }
    }

    /// Converts packed rotation to quaternion
    #[inline]
    pub fn to_quat(&self) -> Quat {
        self.get_rotation()
            .map(|(x, y, z)| {
                // Convert to radians (0, π/2, π, 3π/2)
                let angles = [
                    x as f32 * PI / 2.0,
                    y as f32 * PI / 2.0,
                    z as f32 * PI / 2.0,
                ];

                // Apply rotations in ZYX order
                Quat::from_rotation_z(angles[2])
                    * Quat::from_rotation_y(angles[1])
                    * Quat::from_rotation_x(angles[0])
            })
            .unwrap_or_else(|| Quat::IDENTITY)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Block::Simple (material, _) | Block::Marching (material, _) => *material == 0,
            Block::None => true,
        }
    }

    #[inline]
    pub fn is_marching(&self) -> bool {
        matches!(self, Block::Marching { .. })
    }

    #[inline]
    pub fn texture_coords(&self) -> [f32; 2] {
        let material = self.material();
        match material {
            1 => [0.0, 1.0],
            _ => [0.0, 0.0],
        }
    }

    #[inline]
    pub fn material(&self) -> u16 {
        match self {
            Block::Simple (material,_) | Block::Marching (material,_) => *material,
            Block::None => 0,
        }
    }

    #[inline]
    pub fn set_material(&mut self, material: u16) {
        match self {
            Block::Simple (mat,_)| Block::Marching (mat,_) => {
                *mat = material
            }
            Block::None => {}
        }
    }

    /// Rotates the block around an axis by N 90° steps
    #[inline]
    pub fn rotate(&mut self, axis: Axis, steps: u8) {
        if let Block::Simple (_,rotation) = self {
            let (mask, shift) = match axis {
                Axis::X => (Self::ROT_MASK_X, Self::ROT_SHIFT_X),
                Axis::Y => (Self::ROT_MASK_Y, Self::ROT_SHIFT_Y),
                Axis::Z => (Self::ROT_MASK_Z, Self::ROT_SHIFT_Z),
            };

            let current = (*rotation & mask) >> shift;
            let new_rot = (current + steps) % 4;
            *rotation = (*rotation & !mask) | (new_rot << shift);
        }
    }

    /// Converts quaternion to packed rotation format
    #[inline]
    pub fn quat_to_rotation(rotation: Quat) -> u8 {
        // Extract Euler angles using efficient quaternion conversion
        let (x, y, z) = rotation.to_euler(glam::EulerRot::ZYX);

        // Convert to 2-bit values (0-3) representing 90-degree increments
        let x = ((x.rem_euclid(TAU) / (PI / 2.0)).round() as u8) & 0x3;
        let y = ((y.rem_euclid(TAU) / (PI / 2.0)).round() as u8) & 0x3;
        let z = ((z.rem_euclid(TAU) / (PI / 2.0)).round() as u8) & 0x3;

        // Pack into a single byte
        x | (y << 2) | (z << 4)
    }

    /// Sets all rotation axes at once
    #[inline]
    pub fn set_rotation(&mut self, x: u8, y: u8, z: u8) {
        if let Block::Simple(_,rotation) = self {
            *rotation = (x & 0x3) | ((y & 0x3) << 2) | ((z & 0x3) << 4);
        }
    }

    /// Sets a point in the 3x3x3 density field
    #[inline]
    pub fn set_point(&mut self, x: u8, y: u8, z: u8, value: bool) {
        if let Block::Marching(_,points) = self {
            debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
            let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
            *points = (*points & !(1 << bit_pos)) | ((value as u32) << bit_pos);
        }
    }

    /// Gets a point from the 3x3x3 density field
    #[inline]
    pub fn get_point(&self, x: u8, y: u8, z: u8) -> Option<bool> {
        match self {
            Block::Marching(_,points) => {
                debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
                let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
                Some((*points & (1u32 << bit_pos)) != 0)
            }
            _ => None,
        }
    }

    pub fn get_march(&mut self) -> Option<Block> {
        match self {
            Block::Marching(_,_) => None,
            _ => Some(Self::new_march(self.material(), 0)),
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

/// Axis enumeration for rotation
#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Compact chunk coordinate representation (64 bits)
/// Format: [X:26 (signed), Y:12 (signed), Z:26 (signed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord(u64);
impl Into<u64> for ChunkCoord {
    fn into(self) -> u64 {
        self.0 // Access the inner u64 value
    }
}
#[allow(dead_code)]
impl ChunkCoord {
    // Use bit shifts that are powers of 2 for better optimization
    const Z_SHIFT: u8 = 0;
    const Y_SHIFT: u8 = 26;
    const X_SHIFT: u8 = 38;

    // Masks should match the shift counts
    const Z_MASK: u64 = (1 << 26) - 1;
    const Y_MASK: u64 = (1 << 12) - 1;
    const X_MASK: u64 = (1 << 26) - 1;

    /// Creates a new chunk coordinate
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(Self::pack(x, y, z))
    }

    /// Packs coordinates into a u64
    #[inline]
    pub const fn pack(x: i32, y: i32, z: i32) -> u64 {
        ((x as u64 & Self::X_MASK) << Self::X_SHIFT)
            | ((y as u64 & Self::Y_MASK) << Self::Y_SHIFT)
            | (z as u64 & Self::Z_MASK)
    }

    /// Extracts (x, y, z) coordinates
    #[inline]
    pub const fn unpack(self) -> (i32, i32, i32) {
        (self.x(), self.y(), self.z())
    }

    /// Extracts X coordinate with sign extension
    #[inline]
    pub const fn x(self) -> i32 {
        ((self.0 >> Self::X_SHIFT) as i32)
            .wrapping_shl(6)
            .wrapping_shr(6)
    }

    /// Extracts Y coordinate with sign extension
    #[inline]
    pub const fn y(self) -> i32 {
        ((self.0 >> Self::Y_SHIFT) as i32 & Self::Y_MASK as i32)
            .wrapping_shl(20)
            .wrapping_shr(20)
    }

    /// Extracts Z coordinate with sign extension
    #[inline]
    pub const fn z(self) -> i32 {
        (self.0 as i32 & Self::Z_MASK as i32)
            .wrapping_shl(6)
            .wrapping_shr(6)
    }

    /// Converts to world position (chunk min corner)
    #[inline]
    pub fn to_world_pos(self) -> Vec3 {
        let chunk_size = Chunk::SIZE_I;
        Vec3::new(
            (self.x() * chunk_size) as f32,
            (self.y() * chunk_size) as f32,
            (self.z() * chunk_size) as f32,
        )
    }

    /// Creates from world position
    #[inline]
    pub fn from_world_pos(world_pos: Vec3) -> Self {
        let chunk_size = Chunk::SIZE_I;
        Self::new(
            world_pos.x.div_euclid(chunk_size as f32) as i32,
            world_pos.y.div_euclid(chunk_size as f32) as i32,
            world_pos.z.div_euclid(chunk_size as f32) as i32,
        )
    }
}

/// Represents a chunk of blocks in the world
#[derive(Clone, PartialEq)]
pub struct Chunk {
    pub blocks: [Block; 4096], // Fixed-size array; None = air/empty
    pub dirty: bool,
    pub mesh: Option<GeometryBuffer>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("dirty", &self.dirty)
            .field("is_empty", &self.is_empty())
            .field("blocks", &self.blocks.len())
            .field("has_bind_group", &self.bind_group.is_some())
            .field("has_mesh", &self.mesh.is_some())
            .finish()
    }
}

impl Chunk {
    pub const SIZE: usize = 16;
    pub const SIZE_I: i32 = Self::SIZE as i32;
    pub const VOLUME: usize = Self::SIZE.pow(3); // 4096

    /// Creates an empty chunk (all blocks are `None` = air)
    #[inline]
    pub fn empty() -> Self {
        Self {
            blocks: [Block::None; Self::VOLUME], // Initialize all blocks as empty
            dirty: false,
            mesh: None,
            bind_group: None,
        }
    }

    /// Creates a new filled chunk (all blocks initialized to `Block::new()`)
    #[inline]
    pub fn new() -> Self {
        let mut chunk = Self::empty();
        let new_block = Block::new();

        // Iterate through all possible positions and set blocks
        for x in 0..Self::SIZE {
            for y in 0..Self::SIZE {
                for z in 0..Self::SIZE {
                    let idx = Self::xyz_to_index(x, y, z);
                    chunk.blocks[idx] = new_block.clone();
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

    /// Converts (x, y, z) to array index (0..4095)
    #[inline]
    pub fn xyz_to_index(x: usize, y: usize, z: usize) -> usize {
        (x << 8) | (y << 4) | z // Equivalent to your `pack_position` but as an index
    }
    #[inline]
    pub fn local_to_index(local_pos: Vec3) -> usize {
        debug_assert!(
            local_pos.x >= 0.0 && local_pos.x < Self::SIZE as f32 &&
            local_pos.y >= 0.0 && local_pos.y < Self::SIZE as f32 &&
            local_pos.z >= 0.0 && local_pos.z < Self::SIZE as f32,
            "Local position out of bounds: {:?}",
            local_pos
        );
        let x = local_pos.x as usize;
        let y = local_pos.y as usize;
        let z = local_pos.z as usize;
        Self::xyz_to_index(x, y, z)
    }

    #[inline]
    pub fn get_block(&self, index: usize) -> &Block {
        &self.blocks[index]
    }

    #[inline]
    pub fn get_block_mut(&mut self, index: usize) -> &mut Block {
        &mut self.blocks[index]
    }

    /// Checks if the chunk is completely empty (all blocks are None or empty)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|b| b.is_empty())
    }

    /// Sets a block at a local position
    pub fn set_block(&mut self, index: usize, block: Block) {
        self.blocks[index] =
            if block.is_empty() { Block::None } else { block };
        self.dirty = true;
    }

    /// Converts world position to local chunk coordinates
    #[inline]
    pub fn world_to_local_pos(world_pos: Vec3) -> Vec3 {
        Vec3::new(
            world_pos.x.rem_euclid(Self::SIZE as f32),
            world_pos.y.rem_euclid(Self::SIZE as f32),
            world_pos.z.rem_euclid(Self::SIZE as f32),
        )
    }

    /// Packs local coordinates into u16
    #[inline]
    pub fn pack_position(local_pos: Vec3) -> u16 {
        debug_assert!(
            local_pos.x < (Self::SIZE as u32) as f32
                && local_pos.y < (Self::SIZE as u32) as f32
                && local_pos.z < (Self::SIZE as u32) as f32,
            "Position out of bounds"
        );
        ((local_pos.x as u16) << 8) | ((local_pos.y as u16) << 4) | (local_pos.z as u16)
    }

    /// Unpacks position key to local coordinates
    #[inline]
    pub fn unpack_position(pos: u16) -> Vec3 {
        Vec3::new(
            (((pos >> 8) & 0xF) as u32) as f32,
            (((pos >> 4) & 0xF) as u32) as f32,
            ((pos & 0xF) as u32) as f32,
        )
    }

    pub fn make_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, force: bool) {
        if !force && !self.dirty && self.mesh.is_some() {
            return;
        }

        // Early return if chunk is empty
        if self.is_empty() {
            // If we have an existing mesh but the chunk is now empty, clear it
            if self.mesh.is_some() {
                self.mesh = Some(GeometryBuffer::empty(device));
                self.dirty = false;
            }
            return;
        }

        let mut builder = ChunkMeshBuilder::new();

        for pos in 0..Self::VOLUME {
            let block = self.blocks[pos];
            if block.is_empty() {
                continue;
            }
            
            let local_pos = Self::unpack_position(pos as u16);
            match block {
                Block::Marching(_,points) => {
                    builder.generate_marching_cubes_mesh(points, local_pos);
                }
                _ => {
                    builder.add_cube(local_pos, block.to_quat(), block.texture_coords(), self);
                }
            }
        }

        if let Some(mesh) = &mut self.mesh {
            mesh.update(device, queue, &builder.indices, &builder.vertices);
        } else {
            self.mesh = Some(GeometryBuffer::new(
                device,
                &builder.indices,
                &builder.vertices,
            ));
        }
        self.dirty = false;
    }

    /// Checks if a block position is empty or outside the chunk
    #[inline]
    fn is_block_cull(&self, pos: Vec3) -> bool {
        // Check if position is outside chunk bounds
        if pos.x < 0.0 || pos.y < 0.0 || pos.z < 0.0 ||
           pos.x >= Self::SIZE_I as f32 || 
           pos.y >= Self::SIZE_I as f32 || 
           pos.z >= Self::SIZE_I as f32 
        {
            return true;
        }
        
        let idx = Chunk::local_to_index(pos);
        match self.blocks[idx] {
            Block::None => true,
            block => block.is_empty() || block.is_marching(),
        }
    }

    /// Returns a reference to the mesh if it exists
    #[inline]
    pub fn mesh(&self) -> Option<&GeometryBuffer> {
        self.mesh.as_ref()
    }

    /// Returns a reference to the bind group if it exists
    #[inline]
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }
}

/// Represents the game world containing chunks
#[derive(Debug, Clone)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
}

impl World {
    /// Creates an empty world
    #[inline]
    pub fn empty() -> Self {
        Self {
            chunks: FastMap::with_capacity_and_hasher(
                10_000,
                BuildHasherDefault::<AHasher>::default(),
            ),
            loaded_chunks: HashSet::with_capacity(10_000),
        }
    }

    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    #[inline]
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    #[inline]
    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
        self.chunks.get_mut(&coord)
    }

    #[inline]
    pub fn get_block(&self, world_pos: Vec3) -> &Block {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        let local_pos = Chunk::world_to_local_pos(world_pos);
        let idx = Chunk::local_to_index(local_pos);

        self.chunks.get(&chunk_coord)
            .map(|chunk| chunk.get_block(idx))
            .unwrap_or(&Block::None)
    }

    #[inline]
    pub fn get_block_mut(&mut self, world_pos: Vec3) -> Option<&mut Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        let local_pos = Chunk::world_to_local_pos(world_pos);
            let index = Chunk::local_to_index(local_pos);
        self.chunks.get_mut(&chunk_coord).and_then(|chunk| {
            Some(chunk.get_block_mut(index))
        })
    }

    pub fn set_block(&mut self, world_pos: Vec3, block: Block) {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);

        if !self.chunks.contains_key(&chunk_coord) {
            self.set_chunk(chunk_coord, Chunk::empty());
        }

        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
        let local_pos = Chunk::world_to_local_pos(world_pos);
            let index = Chunk::local_to_index(local_pos);
            if chunk.get_block(index) != &block {
                chunk.set_block(index, block);
            }
        }
    }

    /// Loads a chunk from storage
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoord, force: bool) -> bool {
        unsafe {
            let state = super::get_state();
            let device = &state.render_context.device;
            let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

            // First check if we already have this chunk loaded with the same contents
            let mut chunk = Chunk::empty();
            if force {
                // Load new chunk data
                chunk = match Chunk::load() {
                    Some(c) => c,
                    None => return false,
                };
            }

            if let Some(existing_chunk) = self.get_chunk(chunk_coord) {
                // If the chunk exists and isn't different form the existing one, no need to reload
                if existing_chunk.blocks == chunk.blocks {
                    return false;
                }
            }

            // Create position buffer
            let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Position Buffer"),
                contents: bytemuck::cast_slice(&[
                    <ChunkCoord as Into<u64>>::into(chunk_coord) as u64,
                    0.0 as u64,
                ]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // Create bind group
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: chunk_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: position_buffer.as_entire_binding(),
                }],
                label: Some("chunk_bind_group"),
            });

            self.chunks.insert(chunk_coord, Chunk {
                        bind_group: Some(bind_group),
                        ..chunk
                    });
            self.loaded_chunks.insert(chunk_coord);

            true
        }
    }

    /// Updates loaded chunks based on player position
    pub fn update_loaded_chunks(&mut self, center: Vec3, radius: f32) {
        let center_coord = ChunkCoord::from_world_pos(center);
        let (center_x, center_y, center_z) = center_coord.unpack();
        let radius_i32 = radius as i32;
        let radius_sq = (radius * radius) as i32;

        // Unload distant chunks
        let mut to_unload = Vec::new();
        for &coord in &self.loaded_chunks {
            let (x, y, z) = coord.unpack();
            let dx = x - center_x;
            let dy = y - center_y;
            let dz = z - center_z;

            if dx * dx + dy * dy + dz * dz > radius_sq {
                to_unload.push(coord);
            }
        }

        for coord in to_unload {
            self.unload_chunk(coord);
        }

        // Load new chunks in range
        for dx in -radius_i32..=radius_i32 {
            for dy in -radius_i32..=radius_i32 {
                for dz in -radius_i32..=radius_i32 {
                    if dx * dx + dy * dy + dz * dz > radius_sq {
                        continue;
                    }

                    let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
                    if !self.loaded_chunks.contains(&coord) {
                        self.load_chunk(coord, false);
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

    /// Generates meshes for all dirty chunks
    #[inline]
    pub fn make_chunk_meshes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let timer = std::time::Instant::now();
        let mut chunk_times = super::debug::RunningAverage::default();
        let mut empty_chunks = 0;

        for chunk in self.chunks.values_mut() {
            // Skip empty chunks entirely
            if chunk.is_empty() {
                empty_chunks += 1;
                continue;
            }

            let chunk_timer = std::time::Instant::now();
            chunk.make_mesh(device, queue, false);
            let elapsed_micros = chunk_timer.elapsed().as_micros() as f32;

            chunk_times.add(elapsed_micros.into());
        }

        println!("World mesh generation stats:\nTotal time: {:.2}ms\nChunks avg: {:.2}µs\nTotal cubes: {} (16³ x chunk_count)",
            timer.elapsed().as_secs_f32() * 1000.0, 
            chunk_times.average(), 
            (self.loaded_chunks.len() - empty_chunks) * Chunk::VOLUME
        );
    }

    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for chunk in self.chunks.values() {
            // Skip empty chunks entirely - no mesh or bind group needed
            if chunk.is_empty() {
                continue;
            }
            
            if let (Some(mesh), Some(bind_group)) = (&chunk.mesh, &chunk.bind_group) {
                // Skip if mesh has no indices (shouldn't happen but good to check)
                if mesh.num_indices == 0 {
                    continue;
                }
                
                render_pass.set_bind_group(2, bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
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
    pub fn generate_marching_cubes_mesh(&mut self, points: u32, position: Vec3) {
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
#[derive(Debug, Clone,PartialEq)]
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
