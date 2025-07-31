
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
	pub storage: BlockStorage,
	pub dirty: bool,
	pub final_mesh: bool,
	pub mesh: Option<GeometryBuffer>,
	pub bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockStorage {
	/// Single block type for all positions (most memory efficient)
	Uniform {
		block: Block,
	},
	/// 4-bit indices (2 blocks per byte) for palettes with ≤16 blocks
	/// Uses 2KB for indices + small palette
	Compact {
		palette: Vec<Block>, // Max 16 entries
		indices: Box<[u8; Chunk::VOLUME/2]>, // 4096 positions, 2 per byte
	},
	/// 8-bit indices for larger palettes (up to 256 blocks)
	/// Uses 4KB for indices + larger palette
	Sparse {
		palette: Vec<Block>, // Max 256 entries
		indices: Box<[u8; Chunk::VOLUME]>, // Full index array
	},
}

impl BlockStorage {
	/// Convert to RLE format only if it would save memory
	pub fn to_rle(&self) -> Option<(Vec<Block>, Vec<(u8, u8)>)> {
		let rle = match self {
			BlockStorage::Uniform { block } => {
				let palette = vec![*block];
				let runs = vec![(Chunk::VOLUME as u8, 0)];
				( palette, runs )
			}
			BlockStorage::Compact { palette, indices } => {
				let mut runs = Vec::new();
				let mut current_block = indices[0] >> 4;
				let mut count = 1;
				
				// Process first nibble
				for i in 0..Chunk::VOLUME {
					let index = if i % 2 == 0 {
						indices[i/2] >> 4
					} else {
						indices[i/2] & 0x0F
					};
					
					if index == current_block && count < u8::MAX {
						count += 1;
					} else {
						runs.push((count, current_block));
						current_block = index;
						count = 1;
					}
				}
				
				// Push the last run
				runs.push((count, current_block));
				
				( palette.clone(), runs )
			}
			BlockStorage::Sparse { palette, indices } => {
				let mut runs = Vec::new();
				let mut current_block = indices[0];
				let mut count = 1;
				
				for &block in indices.iter().skip(1) {
					if block == current_block && count < u8::MAX {
						count += 1;
					} else {
						runs.push((count, current_block));
						current_block = block;
						count = 1;
					}
				}
				
				// Push the last run
				runs.push((count, current_block));
				
				( palette.clone(), runs )
			}
		};

		// Calculate memory sizes
		let original_size = match self {
			BlockStorage::Uniform { .. } => std::mem::size_of::<Block>(),
			BlockStorage::Compact { palette, .. } => std::mem::size_of_val(palette) + std::mem::size_of::<[u8; Chunk::VOLUME/2]>(),
			BlockStorage::Sparse { palette, .. } => std::mem::size_of_val(palette) + std::mem::size_of::<[u8; Chunk::VOLUME]>(),
		};
		let rle_size = std::mem::size_of_val(&*rle.0) + std::mem::size_of_val(&*rle.1);

		// Only return RLE if it's smaller
		if rle_size < original_size {
			Some(rle)
		} else {
			None
		}
	}
	
	/// Convert from RLE format to Compact/Sparse storage
	pub fn from_rle(palette: &[Block], runs: &[(u8, u8)]) -> Option<Self> {
		// First determine if we should use Compact or Sparse storage
		// Compact is more efficient when palette size <= 16
		let use_compact = palette.len() <= 16;
		
		if use_compact {
			let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
			let mut pos = 0;
			
			for &(count, index) in runs {
				for _ in 0..count {
					let nibble_pos = pos / 2;
					if pos % 2 == 0 {
						indices[nibble_pos] = (index << 4) | (indices[nibble_pos] & 0x0F);
					} else {
						indices[nibble_pos] = (indices[nibble_pos] & 0xF0) | (index & 0x0F);
					}
					pos += 1;
					
					if pos >= Chunk::VOLUME {
						break;
					}
				}
			}
			
			Some(BlockStorage::Compact {
				palette: palette.to_vec(),
				indices,
			})
		} else {
			let mut indices = Box::new([0u8; Chunk::VOLUME]);
			let mut pos = 0;
			
			for &(count, index) in runs {
				for _ in 0..count {
					if pos >= Chunk::VOLUME {
						break;
					}
					indices[pos] = index;
					pos += 1;
				}
			}
			
			Some(BlockStorage::Sparse {
				palette: palette.to_vec(),
				indices,
			})
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StorageType {
	Uniform = 0,
	Compact = 1,
	Sparse = 2,
	RleCompressed = 3,
	// Add new types here as needed
}
impl StorageType {
	#[inline] pub const fn from_u8(value: u8) -> Option<Self> {
		unsafe { std::mem::transmute(value) }
	}
	pub const fn as_u8(self) -> u8 {
		self as u8
	}
}

impl BlockStorage {
	const MAX_COMPACT_PALETTE_SIZE: usize = Chunk::SIZE;
	const MAX_SPARSE_PALETTE_SIZE: usize = 256;

	/// Creates empty storage (all air blocks)
	pub const fn empty() -> Self {
		Self::Uniform {
			block: Block::default(), // Air block
		}
	}

	/// Creates uniform storage with a single block type
	pub fn uniform(block: Block) -> Self {
		Self::Uniform { block }
	}

	pub const fn to_type(&self) -> StorageType {
		match self {
			Self::Uniform{ .. } => StorageType::Uniform,
			Self::Compact{ .. } => StorageType::Compact,
			Self::Sparse{ .. } => StorageType::Sparse,
		}
	}

	/// Gets the block at the given position
	#[inline]
	pub fn get(&self, index: usize) -> Block {
		match self {
			Self::Uniform { block } => *block,
			Self::Compact { palette, indices } => {
				let byte_idx = index / 2;
				let is_high_nibble = index % 2 == 1;
				let palette_idx = if is_high_nibble {
					(indices[byte_idx] >> 4) & 0x0F
				} else {
					indices[byte_idx] & 0x0F
				};
				palette[palette_idx as usize]
			}
			Self::Sparse { palette, indices } => {
				let palette_idx = indices[index];
				palette[palette_idx as usize]
			},
		}
	}

	/// Sets the block at the given position, automatically handling storage transitions
	pub fn set(&mut self, index: usize, block: Block) {
		match self {
			Self::Uniform { block: current_block } => {
				if *current_block != block {
					// Need to convert from uniform to either compact or sparse
					let mut new_palette = vec![*current_block];
					let new_block_idx = Self::add_to_palette(&mut new_palette, block, Self::MAX_COMPACT_PALETTE_SIZE);
					
					if new_palette.len() <= Self::MAX_COMPACT_PALETTE_SIZE {
						// Convert to compact storage
						let mut indices = Box::new([0u8; 2048]);
						// Set all positions to index 0 (the original uniform block)
						// Then set the specific position to the new block's index
						Self::set_compact_index(&mut indices, index, new_block_idx);
						*self = Self::Compact { palette: new_palette, indices };
					} else {
						// Convert directly to sparse storage
						let mut indices = Box::new([0u8; 4096]);
						indices[index] = new_block_idx;
						*self = Self::Sparse { palette: new_palette, indices };
					}
				}
				// If block is the same as current uniform block, no change needed
			}
			Self::Compact { palette, indices } => {
				let block_idx = Self::add_to_palette(palette, block, Self::MAX_COMPACT_PALETTE_SIZE);
				
				if palette.len() > Self::MAX_COMPACT_PALETTE_SIZE {
					// Convert to sparse storage
					let new_palette = palette.clone();
					let mut new_indices = Box::new([0u8; 4096]);
					
					// Convert all existing compact indices to sparse format
					for i in 0..Chunk::VOLUME {
						let byte_idx = i / 2;
						let is_high_nibble = i % 2 == 1;
						let palette_idx = if is_high_nibble {
							(indices[byte_idx] >> 4) & 0x0F
						} else {
							indices[byte_idx] & 0x0F
						};
						new_indices[i] = palette_idx;
					}
					new_indices[index] = block_idx;
					*self = Self::Sparse { palette: new_palette, indices: new_indices };
				} else {
					// Stay in compact format
					Self::set_compact_index(indices, index, block_idx);
				}
			}
			Self::Sparse { palette, indices } => {
				let block_idx = Self::add_to_palette(palette, block, Self::MAX_SPARSE_PALETTE_SIZE);
				indices[index] = block_idx;
			}
		}
	}

	/// Helper function to set a 4-bit index in compact storage
	#[inline]
	fn set_compact_index(indices: &mut [u8; 2048], position: usize, palette_idx: u8) {
		let byte_idx = position / 2;
		let is_high_nibble = position % 2 == 1;
		
		if is_high_nibble {
			indices[byte_idx] = (indices[byte_idx] & 0x0F) | ((palette_idx & 0x0F) << 4);
		} else {
			indices[byte_idx] = (indices[byte_idx] & 0xF0) | (palette_idx & 0x0F);
		}
	}

	/// Helper function to add a block to a palette, returning its index
	fn add_to_palette(palette: &mut Vec<Block>, block: Block, max_size: usize) -> u8 {
		// Check if block already exists in palette
		if let Some(idx) = palette.iter().position(|&b| b == block) {
			return idx as u8;
		}

		// Add new block to palette if there's space
		if palette.len() < max_size {
			let idx = palette.len();
			palette.push(block);
			idx as u8
		} else {
			// Palette is full, could implement LRU eviction here
			// For now, just return index 0 (air/first block)
			eprintln!("Warning: Palette is full, using fallback block");
			0
		}
	}

	/// Attempts to optimize storage to more efficient formats
	pub fn optimize(&mut self) {
		match self {
			Self::Sparse { palette, indices } => {
				// Try to optimize sparse to compact or uniform
				let mut used_indices = std::collections::HashSet::new();
				for &idx in indices.iter() {
					used_indices.insert(idx);
				}

				if used_indices.len() == 1 {
					// All blocks are the same - convert to uniform
					let block_idx = *used_indices.iter().next().unwrap();
					let block = palette[block_idx as usize];
					*self = Self::Uniform { block };
				} else if used_indices.len() <= Self::MAX_COMPACT_PALETTE_SIZE {
					// Can fit in compact storage
					let mut new_palette = Vec::new();
					let mut index_mapping = std::collections::HashMap::new();

					// Create new compact palette with only used blocks
					for old_idx in used_indices.iter().copied() {
						let new_idx = new_palette.len() as u8;
						new_palette.push(palette[old_idx as usize]);
						index_mapping.insert(old_idx, new_idx);
					}

					// Convert indices to compact format
					let mut new_indices = Box::new([0u8; 2048]);
					for i in 0..Chunk::VOLUME {
						let old_palette_idx = indices[i];
						let new_palette_idx = index_mapping[&old_palette_idx];
						Self::set_compact_index(&mut new_indices, i, new_palette_idx);
					}

					*self = Self::Compact { palette: new_palette, indices: new_indices };
				}
			}
			Self::Compact { palette, indices } => {
				// Try to optimize compact to uniform
				let mut used_indices = std::collections::HashSet::new();
				for i in 0..Chunk::VOLUME {
					let byte_idx = i / 2;
					let is_high_nibble = i % 2 == 1;
					let palette_idx = if is_high_nibble {
						(indices[byte_idx] >> 4) & 0x0F
					} else {
						indices[byte_idx] & 0x0F
					};
					used_indices.insert(palette_idx);
				}

				if used_indices.len() == 1 {
					// All blocks are the same - convert to uniform
					let block_idx = *used_indices.iter().next().unwrap();
					let block = palette[block_idx as usize];
					*self = Self::Uniform { block };
				}
			}
			Self::Uniform { .. } => {
				// Already optimal
			}
		}
	}

	/// Returns the palette (for debugging/inspection)
	pub fn palette(&self) -> Vec<Block> {
		match self {
			Self::Uniform { block } => vec![*block],
			Self::Compact { palette, .. } => palette.clone(),
			Self::Sparse { palette, .. } => palette.clone(),
		}
	}

	/// Returns memory usage statistics
	pub fn memory_usage(&self) -> (usize, &'static str) {
		match self {
			Self::Uniform { .. } => (std::mem::size_of::<Block>(), "Uniform"),
			Self::Compact { palette, .. } => {
				let palette_size = palette.len() * std::mem::size_of::<Block>();
				let indices_size = 2048; // Box<[u8; 2048]>
				(palette_size + indices_size, "Compact")
			}
			Self::Sparse { palette, .. } => {
				let palette_size = palette.len() * std::mem::size_of::<Block>();
				let indices_size = 4096; // Box<[u8; 4096]>
				(palette_size + indices_size, "Sparse")
			},
		}
	}
}

impl std::fmt::Debug for Chunk {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let (memory_usage, storage_type) = self.storage.memory_usage();
		f.debug_struct("Chunk")
			.field("dirty", &self.dirty)
			.field("is_empty", &self.is_empty())
			.field("has_bind_group", &self.bind_group.is_some())
			.field("has_mesh", &self.mesh.is_some())
			.field("storage_type", &storage_type)
			.field("memory_bytes", &memory_usage)
			.finish()
	}
}

#[allow(dead_code)]
impl Chunk {
	pub const SIZE: usize = 16;
	pub const SIZE_I: i32 = Self::SIZE as i32;
	pub const SIZE_F: f32 = Self::SIZE as f32;
	pub const VOLUME: usize = Self::SIZE * Self::SIZE * Self::SIZE; // 4096

	/// Creates an empty chunk (all blocks are air)
	#[inline]
	pub fn empty() -> Self {
		Self {
			storage: BlockStorage::empty(),
			dirty: false,
			final_mesh: false,
			mesh: None,
			bind_group: None,
		}
	}

	/// Creates a new filled chunk (all blocks initialized to `Block::new(<mat>)`)
	#[inline]
	pub fn new(mat: u16) -> Self {
		let block = Block::new(Material(mat));
		Self {
			storage: BlockStorage::uniform(block),
			dirty: true,
			final_mesh: false,
			mesh: None,
			bind_group: None,
		}
	}

	pub fn generate(coord: ChunkCoord, seed: u32) -> Option<Self> {
		if coord.y() > 8i32 { return Some(Self::empty()); }
		if coord.y() <= -2i32 { return Some(Self::new(1u16)); }
		
		let noise_gen = Noise::new(seed);
		let (world_x, world_y, world_z) = coord.unpack_to_worldpos();
		let mut chunk = Self::empty();
		let block = Block::new(Material(2u16));
		
		// Pre-calculate all noise values for this chunk's XZ plane
		for x in 0..Self::SIZE {
			for z in 0..Self::SIZE {
				let pos_x: i32 = world_x + x as i32;
				let pos_z: i32 = world_z + z as i32;
				
				// Get noise value and scale it to a reasonable height range
				let noise: f32 = noise_gen.terrain_noise_2d(pos_x, pos_z);
				let final_noise = noise * (8 * Chunk::SIZE) as f32;
				
				for y in 0..Self::SIZE {
					let pos_y = world_y + y as i32;
					// If this block is under or in terrain height, make it solid
					if pos_y <= final_noise as i32 {
						// Correct block indexing : BlockPosition
						let idx: BlockPosition = (x, y, z).into();
						chunk.set_block(idx.into(), block); // Set to solid
					}
					// Else leave as air
				}
			}
		}
		Some(chunk)
	}

	#[inline]
	pub fn get_block(&self, index: usize) -> Block {
		self.storage.get(index)
	}

	/// Checks if the chunk is completely empty (all blocks are air)
	#[inline]
	pub fn is_empty(&self) -> bool {
		match &self.storage {
			BlockStorage::Uniform { block } => block.is_empty(),
			_ => {
				// For compact/sparse storage, check if all blocks are air
				for i in 0..Self::VOLUME {
					if !self.storage.get(i).is_empty() {
						return false;
					}
				}
				true
			}
		}
	}

	/// Checks if the chunk is completely full (all blocks are not air)
	#[inline]
	pub fn is_full(&self) -> bool {
		match &self.storage {
			BlockStorage::Uniform { block } => !block.is_empty(),
			_ => {
				// For compact/sparse storage, check if all blocks are non-air
				for i in 0..Self::VOLUME {
					if self.storage.get(i).is_empty() {
						return false;
					}
				}
				true
			}
		}
	}

	/// Sets a block at the given index
	pub fn set_block(&mut self, index: usize, block: Block) {
		self.storage.set(index, block);
		self.dirty = true;

		// Periodically optimize storage to avoid bloat
		if matches!(self.storage, BlockStorage::Sparse { .. }) {
			// Only optimize sparse storage periodically to avoid performance hits
			static mut OPTIMIZATION_COUNTER: usize = 0;
			unsafe {
				OPTIMIZATION_COUNTER += 1;
				if OPTIMIZATION_COUNTER % 100 == 0 {
					self.storage.optimize();
				}
			}
		}
	}

	/// Checks if a block position is empty or outside the chunk
	#[inline]
	pub fn is_block_cull(&self, pos: IVec3) -> bool {
		let idx: usize = BlockPosition::from(pos).into();
		self.get_block(idx).is_empty()
	}

	#[inline]
	pub const fn contains_position(&self, pos: IVec3) -> bool {
		// Check if position is outside chunk bounds
		if pos.x < 0
			|| pos.y < 0
			|| pos.z < 0
			|| pos.x >= Self::SIZE_I
			|| pos.y >= Self::SIZE_I
			|| pos.z >= Self::SIZE_I
		{
			return false;
		} else {
			true
		}
	}

	#[inline]
	pub const fn is_border_block(&self, pos: IVec3) -> bool {
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
	#[inline]
	pub const fn mesh(&self) -> Option<&GeometryBuffer> {
		self.mesh.as_ref()
	}

	/// Returns a reference to the bind group if it exists
	#[inline]
	pub const fn bind_group(&self) -> Option<&wgpu::BindGroup> {
		self.bind_group.as_ref()
	}

	/// Forces storage optimization (useful for debugging or after bulk operations)
	pub fn optimize_storage(&mut self) {
		self.storage.optimize();
	}

	/// Returns storage type and memory usage for debugging
	pub fn storage_info(&self) -> (usize, &'static str) {
		self.storage.memory_usage()
	}
}