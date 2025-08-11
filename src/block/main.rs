
use crate::block::math::{self, ChunkCoord, BlockPosition, BlockRotation};
use crate::utils::math::{Noise};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
	/// Chunk::VOLUME/2 * 1 byte + palette.len() * 3 byte
	Compact {
		palette: Vec<Block>, // Max 16 entries
		indices: Box<[u8; Chunk::VOLUME/2]>, // 4096 positions, 2 per byte
	},
	/// 8-bit indices for larger palettes (up to 256 blocks)
	/// Uses 4KB for indices + larger palette
	/// Chunk::VOLUME * 1 byte + palette.len() * 3 byte
	Sparse {
		palette: Vec<Block>, // Max 256 entries
		indices: Box<[u8; Chunk::VOLUME]>, // Full index array
	},
	/// 16-bit indices for very large palettes (up to 4096 blocks)
	/// Uses 8KB for indices + large palette
	/// Chunk::VOLUME * 1.5 byte + palette.len() * 3 byte
	Giant {
		palette: Vec<Block>, // Max 4K entries
		indices: Box<[u8; Chunk::VOLUME * 3 / 2]>, // 12-bit indices
	},
	/// Direct storage for extremely diverse chunks (no palette)
	/// Uses 12KB (4096 * 12 bits per Block)
	/// Chunk::VOLUME * 3 byte
	Zigzag {
		blocks: Box<[Block; Chunk::VOLUME]>, // Direct block storage
	},
	/// RLE compressed, this will be sized depending on the case
	/// size may wary from 32 byte to 8K bytes
	/// runs.len() * 2 byte + palette.len() * 3 byte
	Rle {
		palette: Vec<Block>, // Max 256 entries
		runs: Vec<(u8, u8)>,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StorageType {
	Uniform = 0,
	Compact = 1,
	Sparse = 2,
	Giant = 3,
	Zigzag = 4,
	Rle = 5,
}

impl StorageType {
	#[inline] pub const fn from_u8(value: u8) -> Option<Self> {
		match value {
			0 => Some(Self::Uniform),
			1 => Some(Self::Compact),
			2 => Some(Self::Sparse),
			3 => Some(Self::Giant),
			4 => Some(Self::Zigzag),
			5 => Some(Self::Rle),
			_ => None,
		}
	}
	pub const fn as_u8(self) -> u8 {
		self as u8
	}
}

impl BlockStorage {
	const MAX_SPARSE_PALETTE_SIZE: usize = 256;
	const MAX_GIANT_PALETTE_SIZE: usize = 4096; // 2^12 = 4096

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
			Self::Giant{ .. } => StorageType::Giant,
			Self::Zigzag{ .. } => StorageType::Zigzag,
			Self::Rle { .. } => StorageType::Rle,
		}
	}

	/// Gets the block at the given position
	#[inline]
	pub fn get(&self, index: usize) -> Block {
		match self {
			Self::Uniform { block } => *block,
			Self::Compact { palette, indices } => {
				let palette_idx = Self::get_compact_index(&**indices, index);
				palette[palette_idx as usize]
			}
			Self::Sparse { palette, indices } => {
				palette[indices[index] as usize]
			},
			Self::Giant { palette, indices } => {
				let palette_idx = Self::get_giant_index(&**indices, index);
				palette[palette_idx as usize]
			},
			Self::Zigzag { blocks } => {
				blocks[index]
			},
			Self::Rle { palette, runs } => {
				let mut pos = 0;
				for (block_idx, count) in runs {
					let end_pos = pos + *count as usize;
					if index < end_pos {
						return palette[*block_idx as usize];
					}
					pos = end_pos;
				}
				Block::default() // Fallback to air if index is out of bounds
			},
		}
	}

	/// Sets the block at the given position, automatically handling storage transitions
	pub fn set(&mut self, index: usize, block: Block) {
		match self {
			Self::Uniform { block: current_block } => {
				if *current_block == block { return; }

				let mut new_palette = vec![*current_block];
				let new_block_idx = Self::add_to_palette(&mut new_palette, block) as u8;
				
				if new_palette.len() <= 16 {
					// Convert to compact storage
					let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
					Self::set_compact_index(&mut indices, index, new_block_idx);
					*self = Self::Compact { palette: new_palette, indices };
				} else if new_palette.len() <= Self::MAX_SPARSE_PALETTE_SIZE {
					// Convert to sparse storage
					let mut indices = Box::new([0u8; Chunk::VOLUME]);
					indices[index] = new_block_idx;
					*self = Self::Sparse { palette: new_palette, indices };
				} else if new_palette.len() <= Self::MAX_GIANT_PALETTE_SIZE {
					// Convert to giant storage
					let mut indices = Box::new([0u8; Chunk::VOLUME * 3 / 2]);
					Self::set_giant_index(&mut *indices, index, new_block_idx as u16);
					*self = Self::Giant { palette: new_palette, indices };
				} else {
					// Convert to zigzag storage
					let mut blocks = Box::new([*current_block; Chunk::VOLUME]);
					blocks[index] = block;
					*self = Self::Zigzag { blocks };
				}
			}
			Self::Compact { palette, indices } => {
				let block_idx = Self::add_to_palette(palette, block);
				
				if palette.len() <= 16 { 
					Self::set_compact_index(indices, index, block_idx as u8);
					return;
				}

				if palette.len() <= Self::MAX_SPARSE_PALETTE_SIZE {
					// Convert to sparse storage
					let new_palette = palette.clone();
					let mut new_indices = Box::new([0u8; Chunk::VOLUME]);
					
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
					new_indices[index] = block_idx as u8;
					*self = Self::Sparse { palette: new_palette, indices: new_indices };
				} else if palette.len() <= Self::MAX_GIANT_PALETTE_SIZE {
					// Convert to giant storage
					let new_palette = palette.clone();
					let mut new_indices = Box::new([0u8; Chunk::VOLUME * 3 / 2]);
					
					for i in 0..Chunk::VOLUME {
						let byte_idx = i / 2;
						let is_high_nibble = i % 2 == 1;
						let palette_idx = if is_high_nibble {
							(indices[byte_idx] >> 4) & 0x0F
						} else {
							indices[byte_idx] & 0x0F
						};
						Self::set_giant_index(&mut *new_indices, i, palette_idx as u16);
					}
					Self::set_giant_index(&mut *new_indices, index, block_idx as u16);
					*self = Self::Giant { palette: new_palette, indices: new_indices };
				} else {
					// Convert to zigzag storage
					let mut blocks = Box::new([Block::default(); Chunk::VOLUME]);
					
					for i in 0..Chunk::VOLUME {
						let byte_idx = i / 2;
						let is_high_nibble = i % 2 == 1;
						let palette_idx = if is_high_nibble {
							(indices[byte_idx] >> 4) & 0x0F
						} else {
							indices[byte_idx] & 0x0F
						};
						blocks[i] = palette[palette_idx as usize];
					}
					blocks[index] = block;
					*self = Self::Zigzag { blocks };
				}
			}
			Self::Sparse { palette, indices } => {
				let block_idx = Self::add_to_palette(palette, block);
				
				if palette.len() > Self::MAX_SPARSE_PALETTE_SIZE {
					if palette.len() <= Self::MAX_GIANT_PALETTE_SIZE {
						// Convert to giant storage
						let new_palette = palette.clone();
						let mut new_indices = Box::new([0u8; Chunk::VOLUME * 3 / 2]);
						
						for i in 0..Chunk::VOLUME {
							Self::set_giant_index(&mut *new_indices, i, indices[i] as u16);
						}
						Self::set_giant_index(&mut *new_indices, index, block_idx as u16);
						*self = Self::Giant { palette: new_palette, indices: new_indices };
					} else {
						// Convert to zigzag storage
						let mut blocks = Box::new([Block::default(); Chunk::VOLUME]);
						
						for i in 0..Chunk::VOLUME {
							blocks[i] = palette[indices[i] as usize];
						}
						blocks[index] = block;
						*self = Self::Zigzag { blocks };
					}
				} else {
					indices[index] = block_idx as u8;
				}
			},
			Self::Giant { palette, indices } => {
				let block_idx = Self::add_to_palette(palette, block);
				
				if palette.len() <= Self::MAX_GIANT_PALETTE_SIZE {
					Self::set_giant_index(&mut **indices, index, block_idx as u16);
					return;
				}
				// Convert to zigzag storage
				let mut blocks = Box::new([Block::default(); Chunk::VOLUME]);
				
				for i in 0..Chunk::VOLUME {
					let palette_idx = Self::get_giant_index(&**indices, i);
					blocks[i] = palette[palette_idx as usize];
				}
				blocks[index] = block;
				*self = Self::Zigzag { blocks };
			},
			Self::Zigzag { blocks } => {
				blocks[index] = block;
			},
			Self::Rle { .. } => {
				// Convert to sparse storage when modifying RLE
				let mut new_storage = Self::Sparse {
					palette: Vec::new(),
					indices: Box::new([0u8; Chunk::VOLUME]),
				};
				
				// Copy all blocks to sparse storage
				for i in 0..Chunk::VOLUME {
					new_storage.set(i, self.get(i));
				}
				
				// Now set the new block
				new_storage.set(index, block);
				*self = new_storage;
			},
		}
	}

	/// Helper function to set a 4-bit index in compact storage
	#[inline]
	pub fn set_compact_index(indices: &mut [u8; Chunk::VOLUME/2], position: usize, palette_idx: u8) {
		let byte_idx = position / 2;
		let is_high_nibble = position % 2 == 1;
		
		if is_high_nibble {
			indices[byte_idx] = (indices[byte_idx] & 0x0F) | ((palette_idx & 0x0F) << 4);
		} else {
			indices[byte_idx] = (indices[byte_idx] & 0xF0) | (palette_idx & 0x0F);
		}
	}
	#[inline]
	pub fn get_compact_index(indices: &[u8], position: usize) -> u8 {
		let byte_idx = position / 2;
		if position % 2 == 1 {
			(indices[byte_idx] >> 4) & 0x0F
		} else {
			indices[byte_idx] & 0x0F
		}
	}
	// Helper functions for Giant storage 12-bit packing
	#[inline]
	pub fn get_giant_index(indices: &[u8], position: usize) -> u16 {
		let bit_start = position * 12;
		let byte_start = bit_start / 8;
		let bit_offset = bit_start % 8;
		
		if bit_offset <= 4 {
			// Index fits within 2 bytes
			let low_byte = indices[byte_start] as u16;
			let high_byte = indices[byte_start + 1] as u16;
			let combined = (high_byte << 8) | low_byte;
			(combined >> bit_offset) & 0x0FFF
		} else {
			// Index spans 3 bytes - use u32 to avoid overflow
			let low_byte = indices[byte_start] as u32;
			let mid_byte = indices[byte_start + 1] as u32;
			let high_byte = indices[byte_start + 2] as u32;
			let combined = (high_byte << 16) | (mid_byte << 8) | low_byte;
			((combined >> bit_offset) & 0x0FFF) as u16
		}
	}

	#[inline]
	pub fn set_giant_index(indices: &mut [u8], position: usize, value: u16) {
		let value = value & 0x0FFF; // Ensure 12-bit value
		let bit_start = position * 12;
		let byte_start = bit_start / 8;
		let bit_offset = bit_start % 8;
		
		if bit_offset <= 4 {
			// Index fits within 2 bytes
			let mask = 0x0FFF << bit_offset;
			let current = ((indices[byte_start + 1] as u16) << 8) | (indices[byte_start] as u16);
			let new_value = (current & !mask) | ((value as u16) << bit_offset);
			indices[byte_start] = (new_value & 0xFF) as u8;
			indices[byte_start + 1] = ((new_value >> 8) & 0xFF) as u8;
		} else {
			// Index spans 3 bytes - use u32 to avoid overflow
			let mask = 0x0FFF_u32 << bit_offset;
			let current = ((indices[byte_start + 2] as u32) << 16) | 
						 ((indices[byte_start + 1] as u32) << 8) | 
						 (indices[byte_start] as u32);
			let new_value = (current & !mask) | ((value as u32) << bit_offset);
			indices[byte_start] = (new_value & 0xFF) as u8;
			indices[byte_start + 1] = ((new_value >> 8) & 0xFF) as u8;
			indices[byte_start + 2] = ((new_value >> 16) & 0xFF) as u8;
		}
	}

	/// Helper function to add a block to a palette, returning its index
	#[inline]
	fn add_to_palette(palette: &mut Vec<Block>, block: Block) -> usize {
		// Check if block already exists in palette
		if let Some(idx) = palette.iter().position(|&b| b == block) {
			return idx;
		}
		//Made it so the palette can not be "full" it will just transition to the next storage type if reached
		//println!("Warning: Palette is full, using fallback block");

		let idx = palette.len();
		palette.push(block);
		idx
	}

	/// Attempts to optimize storage to more efficient formats
	pub fn optimize(&mut self) {
		match self {
			Self::Zigzag { blocks } => {
				// Try to convert back to palette-based storage if possible
				let mut unique_blocks = std::collections::HashSet::new();
				for &block in blocks.iter() {
					unique_blocks.insert(block);
				}
				
				if unique_blocks.len() <= Self::MAX_SPARSE_PALETTE_SIZE {
					// Can fit in sparse storage
					let palette: Vec<Block> = unique_blocks.into_iter().collect();
					let mut indices = Box::new([0u8; Chunk::VOLUME]);
					
					for i in 0..Chunk::VOLUME {
						indices[i] = palette.iter().position(|&b| b == blocks[i]).unwrap() as u8;
					}
					
					*self = Self::Sparse { palette, indices };
				}/* else if unique_blocks.len() <= Self::MAX_GIANT_PALETTE_SIZE {
					// Can fit in giant storage
					let palette: Vec<Block> = unique_blocks.into_iter().collect();
					let mut indices = Box::new([0u16; Chunk::VOLUME]);
					
					for i in 0..Chunk::VOLUME {
						indices[i] = palette.iter().position(|&b| b == blocks[i]).unwrap() as u16;
					}
					
					*self = Self::Giant { palette, indices };
				}*/
				// have to make this "u12" not just u16 ...
				// Otherwise stay in Zigzag format
			},
			Self::Giant { palette, indices } => {
				let mut used_indices = std::collections::HashSet::new();
				for i in 0..Chunk::VOLUME {
					let idx = Self::get_giant_index(&**indices, i);
					used_indices.insert(idx);
				}

				if used_indices.len() <= 16 {
					let mut new_palette = Vec::new();
					let mut index_mapping = std::collections::HashMap::new();

					for old_idx in used_indices.iter().copied() {
						let new_idx = new_palette.len() as u8;
						new_palette.push(palette[old_idx as usize]);
						index_mapping.insert(old_idx, new_idx);
					}

					let mut new_indices = Box::new([0u8; Chunk::VOLUME/2]);
					for i in 0..Chunk::VOLUME {
						let old_palette_idx = Self::get_giant_index(&**indices, i);
						let new_palette_idx = index_mapping[&old_palette_idx];
						Self::set_compact_index(&mut new_indices, i, new_palette_idx);
					}

					*self = Self::Compact { palette: new_palette, indices: new_indices };
				} else if used_indices.len() <= Self::MAX_SPARSE_PALETTE_SIZE {
					let mut new_palette = Vec::new();
					let mut index_mapping = std::collections::HashMap::new();

					for old_idx in used_indices.iter().copied() {
						let new_idx = new_palette.len() as u8;
						new_palette.push(palette[old_idx as usize]);
						index_mapping.insert(old_idx, new_idx);
					}

					let mut new_indices = Box::new([0u8; Chunk::VOLUME]);
					for i in 0..Chunk::VOLUME {
						let old_palette_idx = Self::get_giant_index(&**indices, i);
						let new_palette_idx = index_mapping[&old_palette_idx];
						new_indices[i] = new_palette_idx;
					}

					*self = Self::Sparse { palette: new_palette, indices: new_indices };
				}
			},
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
				} else if used_indices.len() <= 16 {
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
					let mut new_indices = Box::new([0u8; Chunk::VOLUME/2]);
					for i in 0..Chunk::VOLUME {
						let old_palette_idx = indices[i];
						let new_palette_idx = index_mapping[&old_palette_idx];
						Self::set_compact_index(&mut new_indices, i, new_palette_idx);
					}

					*self = Self::Compact { palette: new_palette, indices: new_indices };
				}
				// Otherwise stay in Sparse format
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
				// Otherwise stay in Compact format
			}
			Self::Uniform { .. } => {
				// Already optimal
			},
			Self::Rle { .. } => {
				// Already optimal
			},
		}
	}

	/// Returns the palette (for debugging/inspection)
	pub fn palette(&self) -> Vec<Block> {
		match self {
			Self::Uniform { block } => vec![*block],
			Self::Compact { palette, .. } => palette.clone(),
			Self::Sparse { palette, .. } => palette.clone(),
			Self::Giant { palette, .. } => palette.clone(),
			Self::Zigzag { blocks } => {
				let mut unique_blocks = std::collections::HashSet::new();
				for &block in blocks.iter() {
					unique_blocks.insert(block);
				}
				unique_blocks.into_iter().collect()
			},
			Self::Rle { palette, .. } => palette.clone(),
		}
	}

	/// Returns memory usage statistics
	pub fn memory_usage(&self) -> (usize, &'static str) {
		match self {
			Self::Uniform { .. } => (std::mem::size_of::<Block>(), "Uniform"),
			Self::Compact { palette, .. } => {
				let palette_size = palette.len() * std::mem::size_of::<Block>();
				let indices_size = Chunk::VOLUME/2; // Box<[u8; 2048]>
				(palette_size + indices_size, "Compact")
			}
			Self::Sparse { palette, .. } => {
				let palette_size = palette.len() * std::mem::size_of::<Block>();
				let indices_size = Chunk::VOLUME; // Box<[u8; 4096]>
				(palette_size + indices_size, "Sparse")
			},
			Self::Giant { palette, .. } => {
				let palette_size = palette.len() * std::mem::size_of::<Block>();
				let indices_size = Chunk::VOLUME * 3 / 2; // 12 bits per index
				(palette_size + indices_size, "Giant")
			},
			Self::Zigzag { .. } => {
				let blocks_size = Chunk::VOLUME * std::mem::size_of::<Block>();
				(blocks_size, "Zigzag")
			},
			Self::Rle { palette, runs } => {
				let palette_size = palette.len() * std::mem::size_of::<Block>();
				let runsize = runs.len() * 2; // Vec<(u8, u8)>,
				(palette_size + runsize, "Rle comp")
			},
		}
	}
}
impl BlockStorage {
	/// Convert to RLE format only if it would save memory
	pub fn to_rle(&self) -> Option<BlockStorage> {
		let rle: Self = match self {
			BlockStorage::Compact { palette, indices } => {
				let mut runs = Vec::with_capacity(32);
				let mut current_block = indices[0] >> 4;
				let mut count = 0; // Start counting from 0 (stored count = actual - 1)
				
				// Process first nibble
				for i in 1..Chunk::VOLUME {
					let index = if i % 2 == 0 {
						indices[i/2] >> 4
					} else {
						indices[i/2] & 0x0F
					};
					
					if index == current_block && count < u8::MAX - 1 { // Leave room for +1
						count += 1;
					} else {
						runs.push((count, current_block));
						current_block = index;
						count = 0;
					}
				}
				
				// Push the last run
				runs.push((count, current_block));
				
				Self::Rle { palette: palette.clone(), runs }
			}
			BlockStorage::Sparse { palette, indices } => {
				let mut runs = Vec::with_capacity(32);
				let mut current_block = indices[0];
				let mut count = 0; // Start counting from 0 (stored count = actual - 1)
				
				for &block in indices.iter().skip(1) {
					if block == current_block && count < u8::MAX - 1 { // Leave room for +1
						count += 1;
					} else {
						runs.push((count, current_block));
						current_block = block;
						count = 0;
					}
				}
				
				// Push the last run
				runs.push((count, current_block));
				
				Self::Rle { palette: palette.clone(), runs }
			}
			_ => return None,
		};

		// Calculate memory sizes
		let original_size = self.memory_usage();
		let rle_size = rle.memory_usage();

		// Only return RLE if it's smaller
		if rle_size < original_size {
			Some(rle)
		} else {
			None
		}
	}

	/// Convert from RLE format to Compact/Sparse storage
	pub fn from_rle(&self) -> Option<Self> {
		let Self::Rle { palette, runs } = self  else { return None; };

		// First determine if we should use Compact or Sparse storage
		// Compact is more efficient when palette size <= 16
		if palette.len() <= 16 {
			let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
			let mut pos: usize = 0;
			
			for &(count, index) in runs {
				for _ in 0..=count { // =count because stored count is actual-1
					if pos >= Chunk::VOLUME { break; }
					let nibble_pos = pos / 2;
					if pos % 2 == 0 {
						indices[nibble_pos] = ((index << 4) & 0xF0) | (indices[nibble_pos] & 0x0F);
					} else {
						indices[nibble_pos] = (indices[nibble_pos] & 0xF0) | (index & 0x0F);
					}
					pos += 1;
				}
			}
			
			if pos != Chunk::VOLUME {
				println!("not all blocks filled, only: {}", pos);
				return None;
			}
			
			Some(BlockStorage::Compact {
				palette: palette.to_vec(),
				indices,
			})
		} else {
			let mut indices = Box::new([0u8; Chunk::VOLUME]);
			let mut pos = 0;
			
			for &(count, index) in runs {
				for _ in 0..=count { // =count because stored count is actual-1
					if pos >= Chunk::VOLUME { break; }
					indices[pos] = index;
					pos += 1;
				}
			}
			
			if pos != Chunk::VOLUME {
				println!("not all blocks filled, only: {}", pos);
				return None;
			}
			
			Some(BlockStorage::Sparse {
				palette: palette.to_vec(),
				indices,
			})
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

	pub fn generate(coord: ChunkCoord, seed: u32) -> Self {
		if coord.y() > 8i32 { return Self::empty(); }
		if coord.y() <= -2i32 { return Self::new(2u16); }
		
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
		chunk
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
		// Only optimize sparse storage periodically to avoid performance hits
		static mut OPTIMIZATION_COUNTER: usize = 0;
		unsafe {
			OPTIMIZATION_COUNTER += 1;
			if OPTIMIZATION_COUNTER % 100 == 0 {
				self.storage.optimize();
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
