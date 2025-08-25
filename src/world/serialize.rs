
use crate::item::inventory::ItemContainer;
use crate::block::entity::EntityStorage;
use crate::world::region::Region;
use crate::block::math::{BlockRotation, ChunkCoord, LocalPos};
use crate::block::main::{Block, Material, Chunk};
use crate::block::storage::{StorageType, BlockStorage};
use crate::fs::binary::{BinarySerializable, FixedBinarySerializable};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use ahash::AHasher;

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;


//
//
// binary conversions for all kinds of structs and enums just to make the world serialize-able
//
//


impl BinarySerializable for ChunkCoord {
	fn to_binary(&self) -> Vec<u8> {
		self.into_u64().to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let value = u64::from_binary(bytes)?;
		Some(value.into())
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}

impl FixedBinarySerializable for ChunkCoord {
	const BINARY_SIZE: usize = u64::BINARY_SIZE;
}

impl BinarySerializable for LocalPos {
	fn to_binary(&self) -> Vec<u8> {
		self.index().to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let value = u16::from_binary(bytes)?;
		Some(Self::from_index(value))
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}

impl FixedBinarySerializable for LocalPos {
	const BINARY_SIZE: usize = u16::BINARY_SIZE;
}

impl BinarySerializable for Region {
	fn to_binary(&self) -> Vec<u8> {
		self.into_u64().to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let value = u64::from_binary(bytes)?;
		Some(value.into())
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}

impl FixedBinarySerializable for Region {
	const BINARY_SIZE: usize = u64::BINARY_SIZE;
}

impl BinarySerializable for BlockRotation {
	fn to_binary(&self) -> Vec<u8> {
		self.as_u8().to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let value = u8::from_binary(bytes)?;
		Self::from_u8(value)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for BlockRotation {
	const BINARY_SIZE: usize = u8::BINARY_SIZE;
}


impl BinarySerializable for Material {
	fn to_binary(&self) -> Vec<u8> {
		self.inner().to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let value = u16::from_binary(bytes)?;
		Some(Self::from(value))
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for Material {
	const BINARY_SIZE: usize = u16::BINARY_SIZE;
}


impl BinarySerializable for Block {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(Self::binary_size(self));
		data.extend_from_slice(&self.material.to_binary());
		data.extend_from_slice(&self.rotation.to_binary());
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < Self::BINARY_SIZE {
			return None;
		}
		let material = Material::from_binary(&bytes[0..Material::BINARY_SIZE])?;
		let rotation = BlockRotation::from_binary(&bytes[Material::BINARY_SIZE..Material::BINARY_SIZE+BlockRotation::BINARY_SIZE])?;
		
		Some(Block::from(material, rotation))
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for Block {
	const BINARY_SIZE: usize = Material::BINARY_SIZE + BlockRotation::BINARY_SIZE; // Material + BLock Rotation
}


impl BinarySerializable for BlockStorage {
	fn to_binary(&self) -> Vec<u8> {
		let mut data:Vec<u8> = Vec::new();
		match self {
			Self::Uniform { block } => {
				data.push(self.to_type().as_u8());
				data.extend_from_slice(&block.to_binary());
			}
			Self::Compact { palette, indices } => {
				data.push(self.to_type().as_u8());
				// Write palette length
				data.push(palette.len() as u8);
				// Write palette
				for block in palette {
					data.extend_from_slice(&block.to_binary());
				}
				data.extend_from_slice(&indices[..]);
			}
			Self::Sparse { palette, indices } => {
				data.push(self.to_type().as_u8());
				// Write palette length
				data.push(palette.len() as u8);
				// Write palette
				for block in palette {
					data.extend_from_slice(&block.to_binary());
				}
				data.extend_from_slice(&indices[..]);
			},
			Self::Rle { palette, runs } => {
				data.push(StorageType::Rle.as_u8());
				// Write palette length
				data.push(palette.len() as u8);
				// Write palette
				for block in palette {
					data.extend_from_slice(&block.to_binary());
				}
				// Write run count
				data.extend_from_slice(&(runs.len() as u16).to_binary());
				// Write each run (count: u8, index: u8)
				for &(count, index) in runs {
					data.push(count);
					data.push(index);
				}
			},
			Self::Giant { .. } |
			Self::Zigzag { .. } => todo!()
		}
		data
	}

	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() {
			return None;
		}

		fn read_palette(bytes: &[u8], offset: &mut usize) -> Option<Vec<Block>> {
			let palette_len = bytes[*offset] as usize; *offset += 1;
			// Read palette
			let mut palette = Vec::with_capacity(palette_len);
			for _ in 0..palette_len {
				if *offset + Block::BINARY_SIZE > bytes.len() {
					return None;
				}
				let block = Block::from_binary(&bytes[*offset..*offset + Block::BINARY_SIZE])?;
				*offset += Block::BINARY_SIZE;
				palette.push(block);
			}
			Some(palette)
		}
		
		let mut offset = 0;
		let storage_type = StorageType::from_u8(bytes[offset])?; offset += 1;

		if offset >= bytes.len() { return None; }
		match storage_type {
			StorageType::Uniform => {
				if offset + Block::BINARY_SIZE > bytes.len() { return None; }
				let block = Block::from_binary(&bytes[offset..offset + Block::BINARY_SIZE])?;
				Some(Self::Uniform { block })
			}
			StorageType::Compact => {
				let palette = read_palette(bytes, &mut offset)?;

				if offset + Chunk::VOLUME/2 > bytes.len() { return None; }
				let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
				indices.copy_from_slice(&bytes[offset..offset + Chunk::VOLUME/2]);

				Some(Self::Compact { palette, indices })
			}
			StorageType::Sparse => {
				let palette = read_palette(bytes, &mut offset)?;

				if offset + Chunk::VOLUME > bytes.len() { return None; }
				let mut indices = Box::new([0u8; Chunk::VOLUME]);
				indices.copy_from_slice(&bytes[offset..offset + Chunk::VOLUME]);

				Some(Self::Sparse { palette, indices })
			}
			StorageType::Rle => {
				let palette = read_palette(bytes, &mut offset)?;

				// Read run count (u16)
				if offset+2 > bytes.len() { return None; }
				let run_count = u16::from_binary(&bytes[offset..offset+2])? as usize; offset += 2;
				
				// Read runs
				let mut runs = Vec::with_capacity(run_count);
				for _ in 0..run_count {
					if offset + 2 > bytes.len() { return None; }

					let count = bytes[offset];
					let index = bytes[offset+1];
					runs.push((count, index));
					offset += 2;
				}
				// Convert RLE to Compact/Sparse storage
				Some(Self::Rle { palette, runs })
			},
			StorageType::Giant |
			StorageType::Zigzag => todo!()
		}
	}

	fn binary_size(&self) -> usize {
		// Fallback to other storage types if RLE isn't possible
		match self {
			Self::Uniform { block } => {
				1 + // type marker
				block.binary_size() // block
			}
			Self::Compact { palette, .. } => {
				1 + // type marker
				1 + // palette length
				palette.len() * Block::BINARY_SIZE + // palette entries
				Chunk::VOLUME/2 // compact indices array
			}
			Self::Sparse { palette, .. } => {
				1 + // type marker
				1 + // palette length
				palette.len() * Block::BINARY_SIZE + // palette entries
				Chunk::VOLUME // sparse indices array
			}
			// Add this case for RLE-compressed storage
			Self::Rle { palette, runs } => {
				1 + // type marker
				1 + // palette length
				palette.len() * Block::BINARY_SIZE + // palette entries
				2 + // run count
				runs.len() * 2 // runs (each run is 2 bytes: count + index)
			}
			Self::Giant { .. } |
			Self::Zigzag { .. } => todo!()
		}
	}
}

impl BinarySerializable for EntityStorage {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		match self {
			EntityStorage::Empty => {
				data.push(0); // Marker for Empty
			}
			EntityStorage::Sparse(map) => {
				data.push(1); // Marker for Sparse
				data.extend_from_slice(&(map.len() as u16).to_binary()); // Number of entries
				
				for (pos, container) in map {
					data.extend_from_slice(&pos.to_binary());
					data.extend_from_slice(&container.to_binary());
				}
			}
			EntityStorage::Dense(array) => {
				data.push(2); // Marker for Dense
				
				// Write presence markers for all Chunk::VOLUME positions
				for maybe_container in array.iter() {
					if let Some(container) = maybe_container {
						data.push(1);
						data.extend_from_slice(&container.to_binary());
					} else {
						data.push(0);
					}
				}
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		
		let mut offset = 0;
		let storage_type = bytes[offset];
		offset += 1;
		
		match storage_type {
			0 => Some(EntityStorage::Empty),
			1 => {
				// Sparse storage
				if offset + 2 > bytes.len() {
					return None;
				}
				let count = u16::from_binary(&bytes[offset..offset + 2])? as usize;
				offset += 2;
				
				let mut map = FastMap::default();
				
				for _ in 0..count {
					if offset + LocalPos::BINARY_SIZE > bytes.len() {
						return None;
					}
					let pos = LocalPos::from_binary(&bytes[offset..offset + LocalPos::BINARY_SIZE])?;
					offset += LocalPos::BINARY_SIZE;
					
					let container = ItemContainer::from_binary(&bytes[offset..])?;
					offset += container.binary_size();
					
					map.insert(pos, container);
				}
				
				Some(EntityStorage::Sparse(map))
			}
			2 => {
				// Dense storage
				if offset + Chunk::VOLUME > bytes.len() {
					return None;
				}
				
				let mut array = Box::new([const { None }; Chunk::VOLUME]);
				
				// Read container data for positions that have it
				for i in 0..Chunk::VOLUME {
					// Read presence marker
					if bytes[offset] == 1 {
						offset += 1;
						let container = ItemContainer::from_binary(&bytes[offset..])?;
						offset += container.binary_size();
						array[i] = Some(container);
					} else {
						 offset += 1;
					}
				}
				
				Some(EntityStorage::Dense(array))
			}
			_ => None, // Invalid storage type
		}
	}
	
	fn binary_size(&self) -> usize {
		match self {
			EntityStorage::Empty => 1, // Just the type marker
			
			EntityStorage::Sparse(map) => {
				1 + // type marker
				2 + // count (u16)
				map.iter().map(|(pos, container)| {
					pos.binary_size() + container.binary_size()
				}).sum::<usize>()
			}
			
			EntityStorage::Dense(array) => {
				let mut size = 1 + // type marker
					Chunk::VOLUME; // presence markers (1 byte each)
				
				// Add size of all non-empty containers
				for maybe_container in array.iter() {
					if let Some(container) = maybe_container {
						size += container.binary_size();
					}
				}
				
				size
			}
		}
	}
}

impl BinarySerializable for Chunk {
	fn to_binary(&self) -> Vec<u8> {
		let mut data:Vec<u8> = Vec::new();
		// Since storage now contains the palette, we just serialize the storage
		let storage = if let Some(rle) = self.storage().to_rle()
			{ rle.to_binary() } else { self.storage().to_binary() };
		data.extend_from_slice(&storage);
		data.extend_from_slice(&self.entities().to_binary());
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let storage = BlockStorage::from_binary(bytes)?;
		let offset = storage.binary_size();
		let entity = EntityStorage::from_binary(&bytes[offset..])?;
		
		Some(Chunk::from_storage_and_entities(storage, entity))
	}
	fn binary_size(&self) -> usize {
		self.storage().binary_size() + self.entities().binary_size()
	}
}


