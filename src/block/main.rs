
use crate::block::math::{self, ChunkCoord, BlockPosition, BlockRotation};
use crate::hs::math::{Noise};
use crate::render::meshing::GeometryBuffer;
#[allow(unused_imports)]
use crate::ext::stopwatch;
use glam::IVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material(pub u16);
impl Material {
	pub const fn inner(&self) -> u16 {
		self.0
	}
	pub const fn from(val:u16) -> Self {
		Self(val)
	}
}

/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
	pub material: Material,
	pub rotation: BlockRotation, // material, rotation
}

#[allow(dead_code)]
impl Block {
	/// Creates a default empty block
	#[inline] pub const fn default() -> Self {
		Self::new(Material(1))
	}
	/// Creates a new simple block with default material
	#[inline] pub const fn new(material: Material) -> Self {
		Self { material, rotation: BlockRotation::XplusYplus }
	}
	#[inline] pub const fn from(material: Material, rotation:BlockRotation) -> Self {
		Self { material, rotation }
	}
	/// Extracts rotation
	#[inline] pub const fn get_rotation(&self) -> BlockRotation {
		self.rotation
	}
	/// Rotates the block around an axis by N 90° steps
	#[inline]
	pub fn rotate(&mut self, axis: math::AxisBasic, steps: u8) {
		self.set_rotation(self.rotation.rotate(axis, steps));
	}
	#[inline] pub const fn is_empty(&self) -> bool {
		self.material.inner() == 1u16 // this should be reworked as like "is not rendered?"
	}
	#[inline] pub const fn material(&self) -> Material {
		self.material
	}
	#[inline]
	pub fn set_material(&mut self, material: Material) {
		self.material = material;
	}
	/// Sets all rotation axes at once
	#[inline]
	pub fn set_rotation(&mut self, rotation: BlockRotation) {
		self.rotation = rotation;
	}
}

/// Represents a chunk of blocks in the world
#[derive(Clone, PartialEq)]
pub struct Chunk {
	pub palette: Vec<Block>, // Max 256 entries (index 0 = air, indices 1-255 = blocks)
	pub storage: BlockStorage, // Palette indices for each block position
	pub dirty: bool,
	pub final_mesh: bool,
	pub mesh: Option<GeometryBuffer>,
	pub bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockStorage {
	Uniform(u8),             // Single palette index for all blocks
	Sparse(Box<[u8; 4096]>), // Full index array
}

impl BlockStorage {
	/// Gets the palette index at the given position
	#[inline] pub const fn get(&self, index: usize) -> u8 {
		match self {
			BlockStorage::Uniform(palette_idx) => *palette_idx,
			BlockStorage::Sparse(indices) => indices[index],
		}
	}

	/// Sets the palette index at the given position, converting to sparse if needed
	#[inline]
	fn set(&mut self, index: usize, palette_idx: u8) {
		match self {
			BlockStorage::Uniform(current_idx) => {
				if *current_idx != palette_idx {
					// Convert to sparse storage
					let mut indices = Box::new([*current_idx; 4096]);
					indices[index] = palette_idx;
					*self = BlockStorage::Sparse(indices);
				}
			}
			BlockStorage::Sparse(indices) => {
				indices[index] = palette_idx;
			}
		}
	}

	/// Attempts to optimize storage back to uniform if all indices are the same
	#[inline]
	fn try_optimize(&mut self) {
		if let BlockStorage::Sparse(indices) = self {
			let first = indices[0];
			if indices.iter().all(|&idx| idx == first) {
				*self = BlockStorage::Uniform(first);
			}
		}
	}
}

impl std::fmt::Debug for Chunk {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Chunk")
			.field("dirty", &self.dirty)
			.field("is_empty", &self.is_empty())
			.field("has_bind_group", &self.bind_group.is_some())
			.field("has_mesh", &self.mesh.is_some())
			.finish()
	}
}

#[allow(dead_code)]
impl Chunk {
	pub const SIZE: usize = 16;
	pub const SIZE_I: i32 = Self::SIZE as i32;
	pub const SIZE_F: f32 = Self::SIZE as f32;
	pub const VOLUME: usize = Self::SIZE.pow(3); // 4096
	const MAX_PALETTE_SIZE: usize = 256; // Index 0 = air, indices 1-255 = blocks

	/// Creates an empty chunk (all blocks are air)
	#[inline]
	pub fn empty() -> Self {
		Self {
			palette: vec![Block::default()],  // Index 0 is always air
			storage: BlockStorage::Uniform(0u8), // All blocks point to air
			dirty: false,
			final_mesh: false,
			mesh: None,
			bind_group: None,
		}
	}
	/// Creates a new filled chunk (all blocks initialized to `Block::new(<mat>)`)
	#[inline]
	pub fn new(mat: u16) -> Self {
		let mut chunk = Self::empty();
		let new_block = Block::new(Material(mat));
		let idx = chunk.palette_add(new_block);
		chunk.storage = BlockStorage::Uniform(idx);
		chunk.dirty = true;
		chunk
	}

	/// Adds a block to the palette, returning its index
	/// Returns existing index if block already exists
	#[inline]
	fn palette_add(&mut self, block: Block) -> u8 {
		// Air blocks always map to index 0
		if block.is_empty() {
			return 0;
		}

		// Check if block already exists in palette
		if let Some(idx) = self.palette.iter().position(|&b| b == block) {
			return idx as u8;
		}

		// Add new block to palette if there's space
		if self.palette.len() < Self::MAX_PALETTE_SIZE {
			let idx = self.palette.len();
			self.palette.push(block);
			idx as u8
		} else {
			// Palette is full, could implement LRU eviction here
			// For now, just return index 1 (first non-air block)
			eprintln!("Warning: Chunk palette is full, using fallback block");
			1
		}
	}

	/// Removes unused blocks from the palette and updates indices
	fn palette_compact(&mut self) {
		if matches!(self.storage, BlockStorage::Uniform(_)) {
			// For uniform storage, we only need the one block type
			let used_idx = match self.storage {
				BlockStorage::Uniform(idx) => idx,
				_ => unreachable!(),
			};

			if used_idx == 0 {
				// Only air is used
				self.palette = vec![Block::default()];
			} else if used_idx < self.palette.len() as u8 {
				// Compact to just air + the used block
				let used_block = self.palette[used_idx as usize];
				self.palette = vec![Block::default(), used_block];
				self.storage = BlockStorage::Uniform(1);
			}
			return;
		}

		// For sparse storage, find all used palette indices
		let mut used_indices = std::collections::HashSet::new();
		if let BlockStorage::Sparse(indices) = &self.storage {
			for &idx in indices.iter() {
				used_indices.insert(idx);
			}
		}

		// Create new compact palette
		let mut new_palette = Vec::new();
		let mut index_mapping = std::collections::HashMap::new();

		// Air always stays at index 0
		new_palette.push(Block::default());
		index_mapping.insert(0u8, 0u8);

		// Add used blocks in order
		for old_idx in 1..self.palette.len() as u8 {
			if used_indices.contains(&old_idx) {
				let new_idx = new_palette.len() as u8;
				new_palette.push(self.palette[old_idx as usize]);
				index_mapping.insert(old_idx, new_idx);
			}
		}

		// Update storage with new indices
		if let BlockStorage::Sparse(indices) = &mut self.storage {
			for idx in indices.iter_mut() {
				*idx = index_mapping[idx];
			}
		}

		self.palette = new_palette;
		self.storage.try_optimize();
	}

	pub fn generate(coord: ChunkCoord, seed: u32) -> Option<Self> {
		if coord.y() > 8i32 { return Some(Self::empty()); }
		if coord.y() <= -2i32 { return Some(Self::new(1u16)); }
		//let mut stopwatch = stopwatch::RunningAverage::new();
		
		let noise_gen = Noise::new(seed);
		let (world_x, world_y, world_z) = coord.unpack_to_worldpos();
		let mut chunk = Self::empty();
		let block = Block::new(Material(2u16));
		
		// Pre-calculate all noise values for this chunk's XZ plane
		for x in 0..Self::SIZE {
			for z in 0..Self::SIZE {
				let pos_x:i32 = world_x + x as i32;
				let pos_z:i32 = world_z + z as i32;
				
				// Get noise value and scale it to a reasonable height range
				let noise:f32 = noise_gen.terrain_noise_2d(pos_x, pos_z);
				//stopwatch.add(noise as f64);
				let final_noise = noise * (8 * Chunk::SIZE) as f32;
				
				for y in 0..Self::SIZE {
					let pos_y = world_y + y as i32;
					// If this block is under or in terrain height, make it solid
					if pos_y <= final_noise as i32 {
						// Correct block indexing : BlockPosition
						let idx: BlockPosition = (x,y,z).into();
						chunk.set_block(idx.into(), block); // Set to solid
					}
					// Else leave as air
				}
			}
		}
		//println!("stopwatch: {:?}", stopwatch);
		Some(chunk)
	}

	#[inline]
	pub fn get_block(&self, index: usize) -> &Block {
		let palette_idx = self.storage.get(index);
		&self.palette[palette_idx as usize]
	}

	#[inline]
	pub fn get_block_mut(&mut self, index: usize) -> &mut Block {
		let palette_idx = self.storage.get(index);
		&mut self.palette[palette_idx as usize]
	}

	/// Checks if the chunk is completely empty (all blocks are air)
	#[inline]
	pub fn is_empty(&self) -> bool {
		match &self.storage {
			BlockStorage::Uniform(idx) => *idx == 0, // Index 0 is air
			BlockStorage::Sparse(indices) => indices.iter().all(|&idx| idx == 0),
		}
	}
	/// Checks if the chunk is completely full (all blocks are not air)
	#[inline]
	pub fn is_full(&self) -> bool {
		match &self.storage {
			BlockStorage::Uniform(idx) => *idx != 0, // Index 0 is air
			BlockStorage::Sparse(indices) => indices.iter().all(|&idx| idx != 0),
		}
	}

	/// Sets a block at the given index
	pub fn set_block(&mut self, index: usize, block: Block) {
		let palette_idx = self.palette_add(block);
		self.storage.set(index, palette_idx);
		self.dirty = true;

		// Periodically compact the palette to avoid bloat
		if self.palette.len() > 64 {
			self.palette_compact();
		}
	}

	/// Checks if a block position is empty or outside the chunk
	#[inline]
	pub fn is_block_cull(&self, pos: IVec3) -> bool {
		let idx:usize = BlockPosition::from(pos).into();
		let block = *self.get_block(idx);
		block.is_empty()
	}

	#[inline] pub const fn contains_position(&self, pos: IVec3) -> bool {
		// Check if position is outside chunk bounds
		if pos.x < 0
			|| pos.y < 0
			|| pos.z < 0
			|| pos.x >= Self::SIZE_I
			|| pos.y >= Self::SIZE_I
			|| pos.z >= Self::SIZE_I
		{
			return false;
		}
		else { true }
	}
	#[inline] pub const fn is_border_block(&self, pos: IVec3) -> bool {
		// Check if position barely inside the chunk
		if pos.x == 0 || pos.x == 15
			|| pos.y == 0 || pos.y == 15
			|| pos.z == 0 || pos.z == 15
		{
			return true;
		}
		false
	}

	/// Returns a reference to the mesh if it exists
	#[inline] pub const fn mesh(&self) -> Option<&GeometryBuffer> {
		self.mesh.as_ref()
	}

	/// Returns a reference to the bind group if it exists
	#[inline] pub const fn bind_group(&self) -> Option<&wgpu::BindGroup> {
		self.bind_group.as_ref()
	}
}
