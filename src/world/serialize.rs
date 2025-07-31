
use crate::world::manager::WorldData;
use crate::world::main::World;
use crate::block::math::{BlockRotation, ChunkCoord};
use crate::block::main::{Block, Material, StorageType, Chunk, BlockStorage};
use crate::hs::binary::{BinarySerializable, FixedBinarySerializable};
use crate::hs::time::Time;


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
	const BINARY_SIZE: usize = 8;
}


// Implement BinarySerializable for BlockRotation
impl BlockRotation {
	/// Maps `u8` values (0..23) back to `BlockRotation`.
	const BYTE_TO_ROTATION: [BlockRotation; 24] = [
		BlockRotation::XplusYplus, BlockRotation::XplusYminus, BlockRotation::XplusZplus, BlockRotation::XplusZminus,
		BlockRotation::XminusYplus, BlockRotation::XminusYminus, BlockRotation::XminusZplus, BlockRotation::XminusZminus,
		BlockRotation::YplusXplus, BlockRotation::YplusXminus, BlockRotation::YplusZplus, BlockRotation::YplusZminus,
		BlockRotation::YminusXplus, BlockRotation::YminusXminus, BlockRotation::YminusZplus, BlockRotation::YminusZminus,
		BlockRotation::ZplusXplus, BlockRotation::ZplusXminus, BlockRotation::ZplusYplus, BlockRotation::ZplusYminus,
		BlockRotation::ZminusXplus, BlockRotation::ZminusXminus, BlockRotation::ZminusYplus, BlockRotation::ZminusYminus,
	];
	fn as_u8(&self) -> u8 {
		*self as u8
	}
	fn from_u8(byte: u8) -> Option<Self> {
		if byte < Self::BYTE_TO_ROTATION.len() as u8 {
			Some(Self::BYTE_TO_ROTATION[byte as usize])
		} else {
			None
		}
	}
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
	const BINARY_SIZE: usize = 2;
}


impl BinarySerializable for Block {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(Self::binary_size(self));
		data.extend_from_slice(&self.material.to_binary());
		data.push(self.rotation.as_u8());
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < Self::BINARY_SIZE {
			return None;
		}
		let material = Material::from_binary(&bytes[0..Material::BINARY_SIZE])?;
		let rotation = BlockRotation::from_u8(bytes[Material::BINARY_SIZE])?;
		
		Some(Block::from(material, rotation))
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for Block {
	const BINARY_SIZE: usize = Material::BINARY_SIZE + 1; // Material + BLock Rotation
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
				// Write compact indices (2048 bytes)
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
				// Write sparse indices (4096 bytes)
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
			}
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

				// Read compact indices (2048 bytes)
				if offset + Chunk::VOLUME/2 > bytes.len() { return None; }
				let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
				indices.copy_from_slice(&bytes[offset..offset + Chunk::VOLUME/2]);

				Some(Self::Compact { palette, indices })
			}
			StorageType::Sparse => {
				let palette = read_palette(bytes, &mut offset)?;

				// Read sparse indices (4096 bytes)
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
			}
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
		}
	}
}


impl BinarySerializable for Chunk {
	fn to_binary(&self) -> Vec<u8> {
		// Since storage now contains the palette, we just serialize the storage
		self.storage.to_rle().unwrap_or_else(|| self.storage.clone()).to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let storage = BlockStorage::from_binary(bytes)?;
		let storage = BlockStorage::from_rle(&storage).unwrap_or_else(|| storage);
		
		Some(Chunk {
			storage,
			dirty: true,
			final_mesh: false,
			mesh: None,
			bind_group: None,
		})
	}
	fn binary_size(&self) -> usize {
		self.storage.binary_size()
	}
}


// Refine World serialization using the trait system
impl BinarySerializable for World {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Write chunk count
		data.extend_from_slice(&self.chunks.len().to_binary());
		
		// Write each chunk with its coordinates
		for (coord, chunk) in &self.chunks {
			data.extend_from_slice(&coord.to_binary());
			data.extend_from_slice(&chunk.to_binary());
		}
		
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let mut offset = 0;
		
		// Read chunk count
		if bytes.len() < offset + usize::BINARY_SIZE {
			return None;
		}
		let chunk_count = usize::from_binary(&bytes[offset..offset + usize::BINARY_SIZE])?;
		offset += usize::BINARY_SIZE;
		
		let mut world = World::empty();
		
		// Read each chunk
		for _i in 0..chunk_count {
			// Read coordinate
			if bytes.len() < offset + ChunkCoord::BINARY_SIZE {
				return None;
			}
			let coord = ChunkCoord::from_binary(&bytes[offset..offset + ChunkCoord::BINARY_SIZE])?;
			offset += ChunkCoord::BINARY_SIZE;
			
			// Read chunk
			let chunk = Chunk::from_binary(&bytes[offset..])?;
			let chunk_size = chunk.binary_size();
			if offset + chunk_size > bytes.len() { return None; }
			offset += chunk_size;
			
			world.chunks.insert(coord, chunk);
		}
		
		Some(world)
	}
	fn binary_size(&self) -> usize {
		let mut size = usize::BINARY_SIZE; // chunk count
		
		for (coord, chunk) in &self.chunks {
			size += coord.binary_size() + chunk.binary_size();
		}
		
		size
	}
}









// Refine WorldData using the trait system
impl BinarySerializable for WorldData {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		data.extend_from_slice(&self.version.to_binary());
		data.extend_from_slice(&self.creation_date.to_binary());
		data.extend_from_slice(&self.last_opened_date.to_binary());
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let mut offset = 0;
		
		// Read version string
		let version = String::from_binary(&bytes[offset..])?;
		offset += version.binary_size();
		
		if bytes.len() < offset + Time::BINARY_SIZE * 2 {
			return None;
		}
		// Read creation_date
		let creation_date = Time::from_binary(&bytes[offset..offset + Time::BINARY_SIZE])?;
		offset += Time::BINARY_SIZE;
		// Read last_opened_date
		let last_opened_date = Time::from_binary(&bytes[offset..offset + Time::BINARY_SIZE])?;
		
		Some(WorldData {
			version,
			creation_date,
			last_opened_date,
		})
	}
	fn binary_size(&self) -> usize {
		self.version.binary_size() + 
		Time::BINARY_SIZE * 2
	}
}

