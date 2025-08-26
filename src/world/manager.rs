use crate::{
	block::{
		main::Chunk,
		math::{ChunkCoord, LocalPos, REGION_SIZE_U, SUFFIX, PREFIX},
		storage::BlockStorage,
	},
	ext::ptr,
	fs::binary::{BinarySerializable, FixedBinarySerializable},
	world::main::World,
};
use std::{
	collections::HashMap,
	fs::{self, File},
	hash::BuildHasherDefault,
	io::{Error, ErrorKind, Result, Write},
	path::{Path, PathBuf},
};
use ahash::AHasher;

// Type aliases
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

// Constants
pub const TEMP_FILE_SUFFIX: &str = ".tmp";

// Directory Management
// ===================

/// Get the base save directory for the game
pub fn get_save_path() -> PathBuf {
	let base_path = if cfg!(windows) {
		dirs::document_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("My Games")
	} else if cfg!(target_os = "macos") {
		dirs::home_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("Library/Application Support")
	} else {
		dirs::data_local_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("Games")
	};

	base_path.join("Rusticubes")
}

/// Ensure the save directory exists
pub fn ensure_save_dir() -> Result<PathBuf> {
	let path = get_save_path();
	fs::create_dir_all(&path)?;
	Ok(path)
}

/// Get all available world names
pub fn get_world_names() -> Result<Vec<String>> {
	let saves_path = get_save_path().join("saves");
	
	if !saves_path.exists() {
		return Ok(Vec::new());
	}

	let world_names = fs::read_dir(saves_path)?
		.filter_map(|entry| {
			let entry = entry.ok()?;
			let path = entry.path();
			
			if path.is_dir() {
				path.file_name()?.to_str().map(String::from)
			} else {
				None
			}
		})
		.collect();
	
	Ok(world_names)
}

/// Delete a world by name
pub fn del_world(world_name: &str) -> Result<()> {
	let saves_path = get_save_path().join("saves");
	let target_path = saves_path.join(world_name);
	
	if !target_path.exists() {
		return Err(Error::new(
			ErrorKind::NotFound, 
			format!("World '{}' does not exist", world_name)
		));
	}
	
	if !target_path.is_dir() {
		return Err(Error::new(
			ErrorKind::InvalidInput, 
			format!("'{}' is not a directory", world_name)
		));
	}

	fs::remove_dir_all(&target_path)?;
	
	// Update UI on successful deletion
	let state = ptr::get_state();
	state.ui_manager.setup_ui();
	
	Ok(())
}

// World Save Operations
// ====================

/// Save the entire world to disk
pub fn save_entire_world(world_path: &Path) -> Result<()> {
	let game_state = ptr::get_gamestate();
	let world = game_state.world();
	
	if world.chunks.is_empty() {
		return Ok(());
	}
	
	let region_dir = world_path.join("region");
	fs::create_dir_all(&region_dir)?;
	
	let regions = group_chunks_by_region(&world.chunks);
	
	for (region_coord, chunks) in regions {
		save_region(region_coord, chunks, &region_dir)?;
	}
	
	Ok(())
}

/// Save a single region file
fn save_region(
	region_coord: ChunkCoord, 
	chunks: Vec<(ChunkCoord, &Chunk)>, 
	region_dir: &Path
) -> Result<()> {
	// Filter out unfinished chunks and serialize
	let chunk_data: Vec<_> = chunks
		.into_iter()
		.filter(|(_, chunk)| chunk.finished_gen())
		.map(|(coord, chunk)| (coord, chunk.to_binary()))
		.collect();
	
	if chunk_data.is_empty() {
		return Ok(());
	}
	
	let file_path = region_file_path(region_dir, region_coord);
	let temp_path = file_path.with_extension(TEMP_FILE_SUFFIX);
	
	// Load existing chunks if the file exists
	let existing_chunks = load_existing_chunks(&file_path, region_coord)
		.unwrap_or_default();
	
	// Merge existing and new chunks
	let mut all_chunks: HashMap<ChunkCoord, Vec<u8>> = existing_chunks
		.into_iter()
		.map(|(coord, chunk)| (coord, chunk.to_binary()))
		.collect();
	
	// Insert new chunks (this will overwrite existing ones with the same coordinates)
	for (coord, data) in chunk_data {
		all_chunks.insert(coord, data);
	}
	
	// Serialize all chunks
	let serialized_data = serialize_region_data(&all_chunks, region_coord)?;
	
	// Write atomically (temp file -> rename)
	write_atomic(&temp_path, &file_path, &serialized_data)?;
	
	Ok(())
}

/// Group chunks by their region coordinates
fn group_chunks_by_region(chunks: &FastMap<ChunkCoord, Chunk>) -> HashMap<ChunkCoord, Vec<(ChunkCoord, &Chunk)>> {
	let mut regions: HashMap<ChunkCoord, Vec<(ChunkCoord, &Chunk)>> = HashMap::new();
	
	for (coord, chunk) in chunks {
		let region_coord = ChunkCoord::to_region_step(*coord);
		regions.entry(region_coord).or_default().push((*coord, chunk));
	}
	
	regions
}

/// Serialize region data into binary format
fn serialize_region_data(chunks: &HashMap<ChunkCoord, Vec<u8>>, region_coord: ChunkCoord) -> Result<Vec<u8>> {
	let mut data = Vec::with_capacity(1024 * chunks.len());
	
	// Write chunk count
	data.extend_from_slice(&chunks.len().to_binary());
	
	// Write each chunk
	for (coord, chunk_data) in chunks {
		let local_coord = coord_to_local(*coord, region_coord)?;
		let packed_coord = LocalPos::from(local_coord);
		
		data.extend_from_slice(&packed_coord.to_binary());
		data.extend_from_slice(chunk_data);
	}
	
	Ok(data)
}

/// Convert global chunk coordinate to local region coordinate
fn coord_to_local(global_coord: ChunkCoord, region_coord: ChunkCoord) -> Result<(u8, u8, u8)> {
	let region_origin = region_coord.from_region_step();
	let local_x = (global_coord.x() - region_origin.x()) as u8;
	let local_y = (global_coord.y() - region_origin.y()) as u8;
	let local_z = (global_coord.z() - region_origin.z()) as u8;
	
	// Validate coordinates are within region bounds
	if local_x >= REGION_SIZE_U || local_y >= REGION_SIZE_U || local_z >= REGION_SIZE_U {
		return Err(Error::new(
			ErrorKind::InvalidData, 
			format!("Chunk {:?} does not belong to region {:?}", global_coord, region_coord)
		));
	}
	
	Ok((local_x, local_y, local_z))
}

// World Load Operations
// ====================

/// Load the entire world from disk
pub fn load_entire_world(world_path: &Path) -> Result<()> {
	let region_dir = world_path.join("region");
	
	if !region_dir.exists() {
		return Err(Error::new(ErrorKind::NotFound, "World region directory not found"));
	}
	
	let entries = fs::read_dir(&region_dir)?;
	let mut loaded_world = World::empty();
	
	for entry in entries {
		let entry = entry?;
		let path = entry.path();
		
		if path.is_file() && is_valid_region_filename(&path) {
			if let Err(e) = load_region_file(&path, &mut loaded_world) {
				println!("Warning: Failed to load region file {:?}: {}", path, e);
				// Continue loading other regions even if one fails
			}
		}
	}
	
	// Transfer loaded chunks to the game world
	let game_world = ptr::get_gamestate().world_mut();
	for (coord, chunk) in loaded_world.chunks {
		game_world.chunks.insert(coord, chunk);
		game_world.loaded_chunks.insert(coord);
		game_world.create_bind_group(coord);
	}
	
	Ok(())
}

/// Load a single region file
fn load_region_file(path: &Path, world: &mut World) -> Result<()> {
	let region_coord = parse_region_filename(path)?;
	let bytes = fs::read(path)?;

	if bytes.len() < usize::BINARY_SIZE {
		return Err(Error::new(ErrorKind::InvalidData, "Region file too small"));
	}
	
	let chunk_count = usize::from_binary(&bytes[0..usize::BINARY_SIZE])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk count"))?;
	
	let mut cursor = usize::BINARY_SIZE;
	
	for i in 0..chunk_count {
		match load_chunk(&bytes, &mut cursor, region_coord) {
			Ok((coord, chunk)) => {
				world.chunks.insert(coord, chunk);
			}
			Err(e) => {
				println!("Warning: Failed to load chunk {}/{} from {:?}: {}", 
					i + 1, chunk_count, path, e);
				// Continue loading remaining chunks
			}
		}
	}
	
	Ok(())
}

/// Load existing chunks from a region file
fn load_existing_chunks(path: &Path, region_coord: ChunkCoord) -> Result<Vec<(ChunkCoord, Chunk)>> {
	if !path.exists() {
		return Ok(Vec::new());
	}
	
	let bytes = fs::read(path)?;
	
	if bytes.len() < usize::BINARY_SIZE {
		return Ok(Vec::new());
	}
	
	let chunk_count = usize::from_binary(&bytes[0..usize::BINARY_SIZE])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk count"))?;
	
	let mut chunks = Vec::new();
	let mut cursor = usize::BINARY_SIZE;
	
	for _ in 0..chunk_count {
		match load_chunk(&bytes, &mut cursor, region_coord) {
			Ok((coord, chunk)) => chunks.push((coord, chunk)),
			Err(_) => break, // Stop on first error to avoid corruption
		}
	}
	
	Ok(chunks)
}

/// Load a single chunk from binary data
fn load_chunk(bytes: &[u8], cursor: &mut usize, region_coord: ChunkCoord) -> Result<(ChunkCoord, Chunk)> {
	if *cursor + LocalPos::BINARY_SIZE > bytes.len() {
		return Err(Error::new(ErrorKind::InvalidData, "Insufficient data for coordinates"));
	}
	
	let local_coord = LocalPos::from_binary(&bytes[*cursor..*cursor + LocalPos::BINARY_SIZE])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid coordinate data"))?;
	*cursor += LocalPos::BINARY_SIZE;

	if *cursor >= bytes.len() {
		return Err(Error::new(ErrorKind::InvalidData, "No chunk data available"));
	}
	
	let global_coord = region_coord.from_region_step() + local_coord.to_chunk_coord();
	let mut chunk = Chunk::from_binary(&bytes[*cursor..])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk data"))?;
	*cursor += chunk.binary_size();
	
	// Decompress storage if needed
	if let Some(storage) = BlockStorage::from_rle(&chunk.storage()) {
		*chunk.storage_mut() = storage;
	}
	
	Ok((global_coord, chunk))
}

// Utility Functions
// ================

/// Generate the file path for a region file
fn region_file_path(region_dir: &Path, coord: ChunkCoord) -> PathBuf {
	let (x, y, z) = coord.unpack();
	let filename = format!("{}{}.{}.{}{}", PREFIX, x, y, z, SUFFIX);
	region_dir.join(filename)
}

/// Check if a filename is a valid region filename
fn is_valid_region_filename(path: &Path) -> bool {
	path.file_name()
		.and_then(|n| n.to_str())
		.map(|name| name.starts_with(PREFIX) && name.ends_with(SUFFIX))
		.unwrap_or(false)
}

/// Write data to a file atomically using a temporary file
fn write_atomic(temp_path: &Path, final_path: &Path, data: &[u8]) -> Result<()> {
	// Write to temporary file first
	{
		let mut file = File::create(temp_path)?;
		file.write_all(data)?;
		file.sync_all()?; // Ensure data is written to disk
	}
	
	// Atomically move temporary file to final location
	fs::rename(temp_path, final_path)?;
	Ok(())
}

/// Parse region coordinates from filename
fn parse_region_filename(path: &Path) -> Result<ChunkCoord> {
	let filename = path.file_name()
		.and_then(|n| n.to_str())
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid filename"))?;
	
	let coords_str = filename
		.strip_prefix(PREFIX)
		.and_then(|s| s.strip_suffix(SUFFIX))
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid region filename format"))?;
	
	let parts: Vec<&str> = coords_str.split('.').collect();
	if parts.len() != 3 {
		return Err(Error::new(ErrorKind::InvalidData, "Invalid coordinate format"));
	}
	
	let parse_coord = |s: &str| -> Result<i32> {
		s.parse().map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid coordinate value"))
	};
	
	Ok(ChunkCoord::new(
		parse_coord(parts[0])?,
		parse_coord(parts[1])?,
		parse_coord(parts[2])?,
	))
}
