
use crate::world::main::World;
use crate::block::math::{BlockRotation, ChunkCoord};
use crate::block::main::{Block, Chunk, BlockStorage};
use crate::ext::config;
use crate::hs::time;
use std::path::{Path, PathBuf};
use std::io::{self, Read, Write, Result, Error, ErrorKind};
use std::fs::{self, File};
use std::mem;


pub fn get_world_names() -> Result<Vec<String>> {
	let path = config::get_save_path().join("saves");

	let mut folders = Vec::new();

	for entry in std::fs::read_dir(path)? {
		let entry = entry?;
		let path = entry.path();

		if path.is_dir() {
			if let Some(folder_name) = path.file_name() {
				if let Some(name_str) = folder_name.to_str() {
					folders.push(name_str.to_string());
				}
			}
		}
	}

	Ok(folders)
}

pub fn del_world(world_name: &str) {
	// Get the saves path
	let saves_path = match config::get_save_path().join("saves").canonicalize() {
		Ok(p) => p,
		Err(e) => {
			println!("Failed to access saves directory: {}", e);
			return;
		}
	};

	let target_path = saves_path.join(world_name);  // Fixed: use world_name instead of name
	if !target_path.exists() {
		println!("World '{}' does not exist", world_name);
		return;
	}
	if !target_path.is_dir() {
		println!("'{}' is not a directory", world_name);
		return;
	}
	// Try to delete the directory
	match fs::remove_dir_all(&target_path) {
		Ok(_) => {
			println!("Successfully deleted world '{}'", world_name);
			// Refresh UI after successful deletion
			let state = config::get_state();
			state.ui_manager.setup_ui();
		},
		Err(e) => {
			println!("Failed to delete world '{}': {}", world_name, e);
		}
	}
}



#[derive(Debug)]
pub struct WorldData {
	pub version: String,
	pub creation_date: time::Time,
	pub last_opened_date: time::Time,
}

impl WorldData {
	pub fn new() -> Self {
		WorldData {
			version: std::env!("CARGO_PKG_VERSION").to_string(),
			creation_date: time::Time::now(),
			last_opened_date: time::Time::now(),
		}
	}
	
	pub fn update_last_opened(&mut self) {
		self.last_opened_date = time::Time::now();
	}
}

impl WorldData {
	pub fn to_bytes(&self) -> Vec<u8> {
		// Convert version string to bytes (prepend length)
		let version_bytes = self.version.as_bytes();
		let version_len = version_bytes.len() as u32;
		
		// Convert Time structs to bytes
		let creation_bytes = self.creation_date.to_bytes();
		let last_opened_bytes = self.last_opened_date.to_bytes();
		
		// Calculate total size
		let total_size = 4 + version_bytes.len() + creation_bytes.len() + last_opened_bytes.len();
		let mut bytes = Vec::with_capacity(total_size);
		
		// Write version (length + data)
		bytes.extend_from_slice(&version_len.to_le_bytes());
		bytes.extend_from_slice(version_bytes);
		
		// Write time fields
		bytes.extend_from_slice(&creation_bytes);
		bytes.extend_from_slice(&last_opened_bytes);
		
		bytes
	}

	pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
		let mut cursor = 0;
		
		// Read version string
		if bytes.len() < cursor + 4 {
			return Err(Error::new(ErrorKind::InvalidData, "Invalid data length"));
		}
		let version_len = u32::from_le_bytes([bytes[cursor], bytes[cursor+1], bytes[cursor+2], bytes[cursor+3]]) as usize;
		cursor += 4;
		
		if bytes.len() < cursor + version_len + 20 { // 20 = 2 * Time size
			return Err(Error::new(ErrorKind::InvalidData, "Invalid data length"));
		}
		
		let version = String::from_utf8(bytes[cursor..cursor+version_len].to_vec())
			.map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8"))?;
		cursor += version_len;
		
		// Read Time structs
		let creation_date = time::Time::from_bytes(&[
			bytes[cursor], bytes[cursor+1], bytes[cursor+2], bytes[cursor+3],
			bytes[cursor+4], bytes[cursor+5], bytes[cursor+6], bytes[cursor+7],
			bytes[cursor+8], bytes[cursor+9]
		]);
		cursor += 10;
		
		let last_opened_date = time::Time::from_bytes(&[
			bytes[cursor], bytes[cursor+1], bytes[cursor+2], bytes[cursor+3],
			bytes[cursor+4], bytes[cursor+5], bytes[cursor+6], bytes[cursor+7],
			bytes[cursor+8], bytes[cursor+9]
		]);
		
		Ok(WorldData {
			version,
			creation_date,
			last_opened_date,
		})
	}
}

pub fn load_world_data(path: &Path) -> Result<WorldData> {
	let file_path = path.join("world_data.dat");
	match File::open(&file_path) {
		Ok(mut file) => {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes)?;
			WorldData::from_bytes(&bytes)
		},
		Err(e) if e.kind() == ErrorKind::NotFound => {
			let new_data = WorldData::new();
			save_world_data(path, &new_data)?;
			Ok(new_data)
		},
		Err(e) => Err(e),
	}
}

pub fn save_world_data(path: &Path, data: &WorldData) -> Result<()> {
	let file_path = path.join("world_data.dat");
	fs::create_dir_all(&file_path.parent().unwrap())?;
	let temp_path = file_path.with_extension("tmp");
	let bytes = data.to_bytes();
	{
		let mut file = File::create(&temp_path)?;
		file.write_all(&bytes)?;
	}
	fs::rename(temp_path, file_path)?;
	Ok(())
}

pub fn update_world_data(path: &PathBuf) -> Result<()> {
	let mut world_data = load_world_data(path)?;
	let current_version = std::env!("CARGO_PKG_VERSION");
	
	if world_data.version != current_version {
		world_data.version = current_version.to_string();
	}
	world_data.update_last_opened();
	
	save_world_data(path, &world_data)?;
	println!("World data updated");
	
	Ok(())
}

//
//
// binary conversions for all kinds of structs and enums just to make the world serialize-able
//
//

impl Chunk {
	/// Serializes the chunk to a binary format
	pub fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// 1. Write palette (max 256 entries)
		data.push(self.palette.len() as u8);
		for block in &self.palette {
			let block_data = block.to_binary();
			data.extend_from_slice(&block_data);
		}
		
		// 2. Write storage
		match &self.storage {
			BlockStorage::Uniform(idx) => {
				data.push(0); // Storage type marker
				data.push(*idx);
			}
			BlockStorage::Sparse(indices) => {
				data.push(1); // Storage type marker
				data.extend_from_slice(&indices[..]);
			}
		}
		
		data
	}
	
	/// Deserializes the chunk from binary format
	pub fn from_binary(bytes: &[u8]) -> Option<Self> {
		let mut offset = 0;
		// 1. Read palette (with bounds checks)
		let palette_len = bytes.get(0)?;
		offset += 1;
		
		let mut palette = Vec::with_capacity(*palette_len as usize);
		for _ in 0..*palette_len {
			// Ensure remaining bytes are enough for `Block::from_binary`
			if offset >= bytes.len() {
				return None;
			}
			let block = Block::from_binary(&bytes[offset..])?;
			let block_size = block.binary_size();
			// Check if block_size would exceed bounds
			if offset + block_size > bytes.len() {
				return None;
			}
			offset += block_size;
			palette.push(block);
		}
		// 2. Read storage (with bounds checks)
		let storage_type = bytes.get(offset)?;
		offset += 1;
		let storage = match storage_type {
			0 => {
				if offset >= bytes.len() {
					return None;
				}
				let idx = bytes.get(offset)?;
				if (*idx as usize) >= palette.len() {
					return None;
				}
				BlockStorage::Uniform(*idx)
			}
			1 => {
				// Ensure 4096 bytes are available
				if offset + 4096 > bytes.len() {
					return None;
				}
				let mut indices = Box::new([0; 4096]);
				indices.copy_from_slice(&bytes[offset..offset+4096]);
				// Check all indices (early exit if invalid)
				for &index in indices.iter() {
					if (index as usize) >= palette.len() {
						return None;
					}
				}
				BlockStorage::Sparse(indices)
			}
			_ => return None,
		};
		
		Some(Chunk {
			palette,
			storage,
			dirty: true,
			mesh: None,
			bind_group: None,
		})
	}

	/// Returns the size of the binary representation
	pub fn binary_size(&self) -> usize {
		// Palette length byte
		let mut size = 1;
		
		// Palette entries
		for block in &self.palette {
			size += block.binary_size();
		}
		
		// Storage
		size += match &self.storage {
			BlockStorage::Uniform(_) => 2, // type marker + index
			BlockStorage::Sparse(_) => 1 + 4096, // type marker + full array
		};
		
		size
	}
}

impl BlockRotation {
	/// Maps `u8` values (0..23) back to `BlockRotation`.
	const BYTE_TO_ROTATION: [Self; 24] = [
		Self::XplusYplus, Self::XplusYminus, Self::XplusZplus, Self::XplusZminus,
		Self::XminusYplus, Self::XminusYminus, Self::XminusZplus, Self::XminusZminus,
		Self::YplusXplus, Self::YplusXminus, Self::YplusZplus, Self::YplusZminus,
		Self::YminusXplus, Self::YminusXminus, Self::YminusZplus, Self::YminusZminus,
		Self::ZplusXplus, Self::ZplusXminus, Self::ZplusYplus, Self::ZplusYminus,
		Self::ZminusXplus, Self::ZminusXminus, Self::ZminusYplus, Self::ZminusYminus,
	];

	/// Converts to a byte (0..23).
	#[inline]
	pub fn to_byte(self) -> u8 {
		self as u8  // Directly use the enum discriminant
	}

	/// Converts from a byte (returns `None` if invalid).
	#[inline]
	pub fn from_byte(byte: u8) -> Option<Self> {
		if byte < 24 {
			Some(Self::BYTE_TO_ROTATION[byte as usize])
		} else {
			None
		}
	}
}

impl Block {
	/// Serializes the block to a binary format
	pub fn to_binary(&self) -> Vec<u8> {
		match self {
			Block::None => vec![0],
			Block::Simple(material, rotation) => {
				let mut data = vec![1];
				data.extend_from_slice(&material.to_le_bytes());
				data.push(rotation.to_byte());
				data
			}
			Block::Marching(material, density) => {
				let mut data = vec![2];
				data.extend_from_slice(&material.to_le_bytes());
				data.extend_from_slice(&density.to_le_bytes());
				data
			}
		}
	}
	
	/// Deserializes the block from binary format
	pub fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() == 0 { return None; }
		let block_type = bytes.get(0)?;
		
		match block_type {
			0 => Some(Block::None),
			1 => {
				if bytes.len() < 4 { return None; }
				let material = u16::from_le_bytes([bytes[1], bytes[2]]);
				let rotation = BlockRotation::from_byte(bytes[3])?;
				Some(Block::Simple(material, rotation))
			}
			2 => {
				if bytes.len() < 7 { return None; }
				let material = u16::from_le_bytes([bytes[1], bytes[2]]);
				let density = u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
				Some(Block::Marching(material, density))
			}
			_ => None,
		}
	}
	
	/// Returns the size of the binary representation
	pub fn binary_size(&self) -> usize {
		match self {
			Block::None => 1,
			Block::Simple(_, _) => 1 + mem::size_of::<u16>() + 1,
			Block::Marching(_, _) => 1 + mem::size_of::<u16>() + mem::size_of::<u32>(),
		}
	}
}

impl World {
	/// Serializes the world to a binary format
	pub fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// 1. Write chunk count (4 bytes)
		data.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());
		
		// 2. Write each chunk with its coordinates
		for (coord, chunk) in &self.chunks {
			// Write coordinate (8 bytes)
			data.extend_from_slice(&coord.into_u64().to_le_bytes());
			
			// Write chunk data
			let chunk_data = chunk.to_binary();
			data.extend_from_slice(&chunk_data);
		}
		
		data
	}
	
	/// Deserializes the world from binary format
	pub fn from_binary(bytes: &[u8]) -> Option<Self> {
		let mut offset = 0;
		
		// 1. Read chunk count
		if bytes.len() < 4 { return None; }
		let chunk_count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
		offset += 4;
		
		let mut world = World::empty();
		
		// 2. Read each chunk
		for _ in 0..chunk_count {
			// Read coordinate
			if bytes.len() < offset + 8 { return None; }
			let coord = u64::from_le_bytes([
				bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3],
				bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7],
			]);
			offset += 8;
			
			// Read chunk and get actual bytes consumed
			let chunk = Chunk::from_binary(&bytes[offset..])?;
			let consumed_bytes = chunk.binary_size();
			
			// Verify we actually consumed the expected bytes
			if bytes.len() < offset + consumed_bytes {
				return None;
			}
			
			// Insert into world
			world.chunks.insert(coord.into(), chunk);
			offset += consumed_bytes;
		}
		
		Some(world)
	}
	
	/// Saves a single chunk with its coordinates to binary
	pub fn save_chunk(&self, coord: ChunkCoord) -> Option<Vec<u8>> {
		let mut data = Vec::new();
		
		// Write coordinate (8 bytes)
		data.extend_from_slice(&coord.into_u64().to_le_bytes());
		
		// Write chunk data
		if let Some(chunk) = self.chunks.get(&coord) {
			data.extend_from_slice(&chunk.to_binary());
			Some(data)
		} else {
			None
		}
	}
	
	/// Loads a single chunk from binary (returns coordinate and chunk)
	pub fn load_chunk_binary(bytes: &[u8]) -> Option<(ChunkCoord, Chunk)> {
		if bytes.len() < 8 { return None; }
		
		// Read coordinate
		let coord = u64::from_le_bytes([
			bytes[0], bytes[1], bytes[2], bytes[3],
			bytes[4], bytes[5], bytes[6], bytes[7],
		]);
		
		// Read chunk
		let chunk = Chunk::from_binary(&bytes[8..])?;
		
		Some((coord.into(), chunk))
	}
}

impl ChunkCoord {
	/// Serializes the coordinate to bytes (always 8 bytes)
	pub fn to_bytes(&self) -> [u8; 8] {
		self.into_u64().to_le_bytes()
	}
	
	/// Deserializes the coordinate from bytes (must be 8 bytes)
	pub fn from_bytes(bytes: [u8; 8]) -> Self {
		u64::from_le_bytes(bytes).into()
	}
}

// 
// Main world Save - Load functions
// will have to rework them a bit, because the giant world size
// currently a normal world is around 100Kb so it's fine for a single function
// but theoretically it can get up to Billions of Tb, not like anyone will ever go that far
// so yeah processing big worlds with a single function is bad
// 

pub fn save_entire_world(path: &PathBuf) -> Result<()> {
	let game_state = config::get_gamestate();
	let world = game_state.world();
	let world_data = world.to_binary();
	// Ensure safe writing
	let world_dir = path.join("world");
	fs::create_dir_all(&world_dir)?;
	// Create correct paths
	let file_path = world_dir.join("data.dat");
	let temp_path = file_path.with_extension("tmp");
	// Write to temp file first
	{
		let mut file = File::create(&temp_path)?;
		file.write_all(&world_data)?;
	}
	// Atomic rename
	fs::rename(temp_path, &file_path)?;
	
	Ok(())
}

pub fn load_entire_world(path: &PathBuf) -> Result<()> {
	let file_path = path.join("world").join("data.dat");
	// Check if file exists and get its size
	if !file_path.exists() {
		return Err(Error::new(ErrorKind::NotFound, "World file not found"));
	}    
	let mut file = File::open(&file_path)?;
	let mut bytes = Vec::new();
	file.read_to_end(&mut bytes)?;
	// Verify we have at least the chunk count (4 bytes)
	if bytes.len() < 4 {
		return Err(Error::new(ErrorKind::InvalidData, "Invalid world data: too short"));
	}
	// Try to deserialize
	let loaded_world = World::from_binary(&bytes).ok_or_else(|| {
		Error::new(ErrorKind::InvalidData, "Failed to deserialize world")
	})?;
	// Apply the loaded world
	config::get_gamestate().world_change(loaded_world);
	config::get_gamestate().world_mut().remake_rendering();
		
	Ok(())
}
