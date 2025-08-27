
use crate::block::main::Block;
use crate::block::main::Chunk;


#[derive(Debug, Clone, PartialEq)]
pub enum BlockStorage {
	/// Single block type for all positions (most memory efficient)
	Uniform {
		block: Block,
	},
	/// 4-bit indices (2 blocks per byte) for palettes with ≤16 blocks
	/// Uses a small for indices + small palette
	Compact {
		palette: Vec<Block>, // Max 16 entries
		indices: Box<[u8; Chunk::VOLUME/2]>, // 2 per byte
	},
	/// 8-bit indices for larger palettes (up to 256 blocks)
	/// Uses a moderate for indices + big palette
	Sparse {
		palette: Vec<Block>, // Max 256 entries
		indices: Box<[u8; Chunk::VOLUME]>, // Full index array
	},
	/// 16-bit indices for very large palettes (up to 4096 blocks)
	/// Uses a lot for indices + large palette
	Giant {
		palette: Vec<Block>, // Max 4K entries
		indices: Box<[u8; Chunk::VOLUME * 3 / 2]>, // 12-bit indices
	},
	/// Direct storage for extremely diverse chunks (no palette)
	/// Uses 3 bytes for each block
	Zigzag {
		blocks: Box<[Block; Chunk::VOLUME]>, // Direct block storage
	},
	/// RLE compressed, this will be sized depending on the case
	/// size may wary
	Rle {
		palette: Vec<Block>,
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
	#[inline] pub const fn as_u8(self) -> u8 {
		self as u8
	}
}

impl BlockStorage {
	const COMPACT_PALETTE_SIZE: usize = 16; // 0xF
	const SPARSE_PALETTE_SIZE: usize = 256; // 0xFF
	const GIANT_PALETTE_SIZE: usize = 4096; // 0xFFF
											// 0xFFFF = 2^16 aka 32K.. what is currently the chunk size so nothing can be more than that

	/// Creates empty storage (all air blocks)
	#[inline] pub const fn empty() -> Self {
		Self::Uniform {
			block: Block::default(), // Air block
		}
	}

	/// Creates uniform storage with a single block type
	#[inline] pub fn uniform(block: Block) -> Self {
		Self::Uniform { block }
	}

	#[inline] pub const fn to_type(&self) -> StorageType {
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
				
				if new_palette.len() <= Self::COMPACT_PALETTE_SIZE {
					// Convert to compact storage
					let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
					Self::set_compact_index(&mut indices, index, new_block_idx);
					*self = Self::Compact { palette: new_palette, indices };
				} else if new_palette.len() <= Self::SPARSE_PALETTE_SIZE {
					// Convert to sparse storage
					let mut indices = Box::new([0u8; Chunk::VOLUME]);
					indices[index] = new_block_idx;
					*self = Self::Sparse { palette: new_palette, indices };
				} else if new_palette.len() <= Self::GIANT_PALETTE_SIZE {
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
				
				if palette.len() <= Self::COMPACT_PALETTE_SIZE { 
					Self::set_compact_index(indices, index, block_idx as u8);
					return;
				} else if palette.len() <= Self::SPARSE_PALETTE_SIZE {
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
				} else if palette.len() <= Self::GIANT_PALETTE_SIZE {
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
				
				if palette.len() <= Self::SPARSE_PALETTE_SIZE {
					indices[index] = block_idx as u8;
				} else if palette.len() <= Self::GIANT_PALETTE_SIZE {
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
			},
			Self::Giant { palette, indices } => {
				let block_idx = Self::add_to_palette(palette, block);
				
				if palette.len() <= Self::GIANT_PALETTE_SIZE {
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
				
				if unique_blocks.len() <= Self::SPARSE_PALETTE_SIZE {
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

				if used_indices.len() <= Self::COMPACT_PALETTE_SIZE {
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
				} else if used_indices.len() <= Self::SPARSE_PALETTE_SIZE {
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
				} else if used_indices.len() <= Self::COMPACT_PALETTE_SIZE {
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
	#[inline] pub fn palette(&self) -> Vec<Block> {
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
		let rle = match self {
			BlockStorage::Uniform { block } => {
				// For uniform storage, create a single run covering the entire chunk
				Self::Rle { 
					palette: vec![*block], 
					runs: vec![(0, (Chunk::VOLUME - 1) as u8)] // (palette_index, count-1)
				}
			}
			BlockStorage::Compact { palette, indices } => {
				let mut runs = Vec::with_capacity(32);
				let mut current_block_idx = Self::get_compact_index(&**indices, 0);
				let mut count = 0u8;
				
				for i in 1..Chunk::VOLUME {
					let block_idx = Self::get_compact_index(&**indices, i);
					
					if block_idx == current_block_idx && count < u8::MAX {
						count += 1;
					} else {
						runs.push((current_block_idx, count));
						current_block_idx = block_idx;
						count = 0;
					}
				}
				
				// Push the last run
				runs.push((current_block_idx, count));
				
				Self::Rle { palette: palette.clone(), runs }
			}
			BlockStorage::Sparse { palette, indices } => {
				let mut runs = Vec::with_capacity(32);
				let mut current_block_idx = indices[0];
				let mut count = 0u8;
				
				for &block_idx in indices.iter().skip(1) {
					if block_idx == current_block_idx && count < u8::MAX {
						count += 1;
					} else {
						runs.push((current_block_idx, count));
						current_block_idx = block_idx;
						count = 0;
					}
				}
				
				// Push the last run
				runs.push((current_block_idx, count));
				
				Self::Rle { palette: palette.clone(), runs }
			}
			BlockStorage::Giant { palette, indices } => {
				let mut runs = Vec::with_capacity(32);
				let mut current_block_idx = Self::get_giant_index(&**indices, 0) as u8;
				let mut count = 0u8;
				
				for i in 1..Chunk::VOLUME {
					let block_idx = Self::get_giant_index(&**indices, i) as u8;
					
					if block_idx == current_block_idx && count < u8::MAX {
						count += 1;
					} else {
						runs.push((current_block_idx, count));
						current_block_idx = block_idx;
						count = 0;
					}
				}
				
				// Push the last run
				runs.push((current_block_idx, count));
				
				Self::Rle { palette: palette.clone(), runs }
			}
			BlockStorage::Zigzag { blocks } => {
				// Build palette from unique blocks
				let mut palette = Vec::new();
				let mut block_to_index = std::collections::HashMap::new();
				
				for &block in blocks.iter() {
					if !block_to_index.contains_key(&block) {
						block_to_index.insert(block, palette.len() as u8);
						palette.push(block);
					}
				}
				
				// Create runs
				let mut runs = Vec::with_capacity(32);
				let mut current_block_idx = block_to_index[&blocks[0]];
				let mut count = 0u8;
				
				for &block in blocks.iter().skip(1) {
					let block_idx = block_to_index[&block];
					
					if block_idx == current_block_idx && count < u8::MAX {
						count += 1;
					} else {
						runs.push((current_block_idx, count));
						current_block_idx = block_idx;
						count = 0;
					}
				}
				
				// Push the last run
				runs.push((current_block_idx, count));
				
				Self::Rle { palette, runs }
			}
			BlockStorage::Rle { .. } => {
				// Already RLE, return None
				return None;
			}
		};

		// Calculate memory sizes
		let original_size = self.memory_usage().0;
		let rle_size = rle.memory_usage().0;

		// Only return RLE if it's significantly smaller (at least 10% savings)
		if rle_size < original_size * 9 / 10 {
			Some(rle)
		} else {
			None
		}
	}

	/// Convert from RLE format to the most appropriate storage format
	pub fn from_rle(&self) -> Option<Self> {
		let Self::Rle { palette, runs } = self else { return None; };

		// Validate that runs cover the entire chunk
		let total_blocks: usize = runs.iter().map(|(_, count)| *count as usize + 1).sum();
		if total_blocks != Chunk::VOLUME {
			println!("RLE runs don't cover entire chunk: {} blocks instead of {}", total_blocks, Chunk::VOLUME);
			return None;
		}

		// Determine the best storage format based on palette size
		if palette.len() == 1 {
			// Single block type - use uniform
			Some(BlockStorage::Uniform { block: palette[0] })
		} else if palette.len() <= Self::COMPACT_PALETTE_SIZE {
			// Use compact storage
			let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
			let mut pos = 0;
			
			for &(palette_idx, count) in runs {
				for _ in 0..=count { // count is stored as actual_count - 1
					if pos >= Chunk::VOLUME { break; }
					Self::set_compact_index(&mut indices, pos, palette_idx);
					pos += 1;
				}
			}
			
			Some(BlockStorage::Compact {
				palette: palette.clone(),
				indices,
			})
		} else if palette.len() <= Self::SPARSE_PALETTE_SIZE {
			// Use sparse storage
			let mut indices = Box::new([0u8; Chunk::VOLUME]);
			let mut pos = 0;
			
			for &(palette_idx, count) in runs {
				for _ in 0..=count { // count is stored as actual_count - 1
					if pos >= Chunk::VOLUME { break; }
					indices[pos] = palette_idx;
					pos += 1;
				}
			}
			
			Some(BlockStorage::Sparse {
				palette: palette.clone(),
				indices,
			})
		} else if palette.len() <= Self::GIANT_PALETTE_SIZE {
			// Use giant storage
			let mut indices = Box::new([0u8; Chunk::VOLUME * 3 / 2]);
			let mut pos = 0;
			
			for &(palette_idx, count) in runs {
				for _ in 0..=count { // count is stored as actual_count - 1
					if pos >= Chunk::VOLUME { break; }
					Self::set_giant_index(&mut *indices, pos, palette_idx as u16);
					pos += 1;
				}
			}
			
			Some(BlockStorage::Giant {
				palette: palette.clone(),
				indices,
			})
		} else {
			// Too many unique blocks - use zigzag storage
			let mut blocks = Box::new([Block::default(); Chunk::VOLUME]);
			let mut pos = 0;
			
			for &(palette_idx, count) in runs {
				let block = palette[palette_idx as usize];
				for _ in 0..=count { // count is stored as actual_count - 1
					if pos >= Chunk::VOLUME { break; }
					blocks[pos] = block;
					pos += 1;
				}
			}
			
			Some(BlockStorage::Zigzag { blocks })
		}
	}
}
