
use crate::{
	block::{
		main::Chunk,
		math::{ChunkCoord, LocalPos},
		storage::BlockStorage
	},
	ext::ptr, world::main::World,
	utils::time::Time,
	fs::binary::{BinarySerializable, FixedBinarySerializable},
	world::region::Region,
};
use std::{
	fs::{self, File},
	collections::HashMap,
	hash::BuildHasherDefault,
	io::{Error, ErrorKind, Read, Result, Write},
	path::{Path, PathBuf},
};
use ahash::AHasher;
// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

#[inline]
pub fn get_save_path() -> PathBuf {
	let mut path = if cfg!(windows) {
		dirs::document_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("My Games")
	} else if cfg!(target_os = "macos") {
		dirs::home_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("Library/Application Support")
	} else {
		// Linux and others
		dirs::data_local_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("Games")
	};

	path.push("Rusticubes");
	path
}
#[inline]
pub fn ensure_save_dir() -> std::io::Result<PathBuf> {
	let path = get_save_path();
	std::fs::create_dir_all(&path)?;
	Ok(path)
}

pub fn get_world_names() -> Result<Vec<String>> {
	let path = get_save_path().join("saves");

	let mut folders = Vec::new();

	for entry in std::fs::read_dir(path)? {
		let entry = entry?;
		let path = entry.path();

		if !path.is_dir() { continue; }

		let Some(folder_name) = path.file_name() else { continue; };

		let Some(name_str) = folder_name.to_str() else { continue; };
		
		folders.push(name_str.to_string());
	}

	Ok(folders)
}

pub fn del_world(world_name: &str) {
	// Get the saves path
	let saves_path = match get_save_path().join("saves").canonicalize() {
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
			let state = ptr::get_state();
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
	pub creation_date: Time,
	pub last_opened_date: Time,
}
impl WorldData {
	pub fn new() -> Self {
		WorldData {
			version: std::env!("CARGO_PKG_VERSION").to_string(),
			creation_date: Time::now(),
			last_opened_date: Time::now(),
		}
	}
	pub fn update_last_opened(&mut self) {
		self.last_opened_date = Time::now();
	}
}
pub fn load_world_data(path: &Path) -> Result<WorldData> {
	let file_path = path.join("world_data.dat");
	match File::open(&file_path) {
		Ok(mut file) => {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes)?;
			WorldData::from_binary(&bytes).ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "Invalid world data"))
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
	let bytes = data.to_binary();
	{
		let mut file = File::create(&temp_path)?;
		file.write_all(&bytes)?;
	}
	fs::rename(temp_path, file_path)?;
	Ok(())
}
pub fn update_world_data(path: &PathBuf) -> Result<WorldData> {
	let mut world_data = load_world_data(path)?;
	let current_version = std::env!("CARGO_PKG_VERSION");
	
	if world_data.version != current_version {
		world_data.version = current_version.to_string();
	}
	world_data.update_last_opened();
	
	save_world_data(path, &world_data)?;
	
	Ok(world_data)
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


// 
// Main world Save - Load functions
// will have to rework them a bit, because the giant world size
// currently a normal world is around 300Kb so it's fine for a single function
// but theoretically it can get up to Billions of Tb, not like anyone will ever go that far
// so yeah processing big worlds with a single function is bad
// 

// Constants should be at module level and use consistent naming
const TEMP_FILE_SUFFIX: &str = ".tmp";

use crossbeam::thread;
use std::sync::mpsc;

// Add this structure to track save operations
#[derive(Clone)]
pub struct SaveOperation {
	region_coord: Region,
	chunk_data: Vec<(ChunkCoord, Vec<u8>)>, // Serialized chunk data instead of cloned chunks
	file_path: PathBuf,
}

/// Saves the entire world to disk, organizing chunks into region files
pub fn save_entire_world(world_path: &Path) -> Result<()> {
	let game_state = ptr::get_gamestate();
	let world = game_state.world();
	
	// Early exit if no chunks to save
	if world.chunks.is_empty() {
		return Ok(());
	}
	
	// Group chunks by their containing regions
	let regions = group_chunks_by_region(&world.chunks);
	
	// Ensure the region directory exists
	let region_dir = world_path.join("region");
	fs::create_dir_all(&region_dir)?;
	
	// Prepare save operations (serialize data instead of cloning chunks)
	let mut save_operations = Vec::new();
	
	for (region_coord, chunks) in regions {
		// Serialize chunks instead of cloning them to avoid stack overflow
		let mut chunk_data = Vec::new();
		
		for (coord, chunk) in chunks {
			if chunk.finished_gen() {
				// Serialize the chunk to bytes instead of cloning
				chunk_data.push((coord, chunk.to_binary()));
			}
		}
		
		if !chunk_data.is_empty() {
			let file_path = region_file_path(&region_dir, region_coord);
			save_operations.push(SaveOperation {
				region_coord,
				chunk_data,
				file_path,
			});
		}
	}
	
	// Use crossbeam's scoped threads for safe borrowing
	thread::scope(|s| {
		let (sender, receiver) = mpsc::channel();
		
		// Spawn worker threads
		let num_workers = std::cmp::min(save_operations.len(), 4); // Limit to 4 threads
		let operations_per_worker = (save_operations.len() + num_workers - 1) / num_workers;
		
		for chunk in save_operations.chunks(operations_per_worker) {
			let sender = sender.clone();
			let operations = chunk.to_vec();
			
			s.spawn(move |_| {
				for operation in operations {
					let result = save_region_threaded(operation);
					if sender.send(result).is_err() {
						break; // Main thread dropped receiver
					}
				}
			});
		}
		
		// Drop the original sender so receiver knows when all threads are done
		drop(sender);
		
		// Collect results from worker threads
		let mut errors = Vec::new();
		for result in receiver {
			if let Err(e) = result {
				errors.push(e);
			}
		}
		
		// Report any errors (but don't fail the entire operation)
		for error in errors {
			println!("Save error: {}", error);
		}
	}).unwrap();
	
	Ok(())
}

/// Thread-safe version of save_region that owns its data
fn save_region_threaded(operation: SaveOperation) -> Result<()> {
	let SaveOperation { region_coord, chunk_data, file_path } = operation;
	
	let temp_path = file_path.with_extension(TEMP_FILE_SUFFIX);
	
	// Load existing chunks first (if any)
	let existing_chunks = if file_path.exists() {
		match load_existing_chunks(&file_path, region_coord) {
			Ok(chunks) => chunks,
			Err(e) => {
				println!("Failed to load existing chunks, starting fresh: {}", e);
				Vec::new()
			}
		}
	} else {
		Vec::new()
	};
	
	// Merge chunks (new chunks overwrite existing ones)
	let mut merged: HashMap<ChunkCoord, Vec<u8>> = HashMap::new();
	
	// Add existing chunks (as serialized data)
	for (coord, chunk) in existing_chunks {
		merged.insert(coord, chunk.to_binary());
	}
	
	// Add/overwrite with new chunks
	for (coord, data) in chunk_data {
		merged.insert(coord, data);
	}
	
	// Convert to vec for serialization
	let final_chunks: Vec<(ChunkCoord, Vec<u8>)> = merged.into_iter().collect();
	
	// Serialize and write atomically
	let data = serialize_region_data(&final_chunks, region_coord)?;
	write_atomic(&temp_path, &file_path, &data)?;
	
	Ok(())
}

/// Serializes region data to binary format using pre-serialized chunk data
fn serialize_region_data(chunks: &[(ChunkCoord, Vec<u8>)], region_coord: Region) -> Result<Vec<u8>> {
	let mut data = Vec::with_capacity(1024 * chunks.len()); // Better capacity estimation
	
	// Write chunk count
	data.extend_from_slice(&chunks.len().to_binary());
	/// Converts global chunk coordinate to local coordinate within region
	fn coord_to_local(global_coord: ChunkCoord, region_coord: Region) -> (u8,u8,u8) {
		let region_origin = region_coord.to_chunk_coord();
		(
			(global_coord.x() - region_origin.x()) as u8,
			(global_coord.y() - region_origin.y()) as u8,
			(global_coord.z() - region_origin.z()) as u8,
		)
	}
	// Write each chunk
	for (coord, chunk_data) in chunks {
		let local_coord = coord_to_local(*coord, region_coord);
		let packed_coord = LocalPos::from(local_coord);
		data.extend_from_slice(&packed_coord.to_binary());
		data.extend_from_slice(chunk_data);
	}
	
	Ok(data)
}

// The rest of your functions remain mostly the same, but here are the key changes:

/// Loads existing chunks from a region file (optimized to avoid deep cloning)
fn load_existing_chunks(path: &Path, region_coord: Region) -> Result<Vec<(ChunkCoord, Chunk)>> {
	let bytes = fs::read(path)?;
	let mut chunks = Vec::new();
	
	if bytes.len() < 8 {
		return Ok(chunks); // Empty file or invalid
	}
	
	let chunk_count = usize::from_binary(&bytes[0..8])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid usize data"))?;
	let mut cursor = 8;
	
	for _ in 0..chunk_count {
		if cursor >= bytes.len() {
			break; // Prevent out-of-bounds access
		}
		
		if let Ok((coord, chunk)) = load_chunk(&bytes, &mut cursor, region_coord) {
			chunks.push((coord, chunk));
		} else {
			break;
		}
	}
	
	Ok(chunks)
}

/// Groups chunks by their containing region coordinates (borrows instead of cloning)
fn group_chunks_by_region(chunks: &FastMap<ChunkCoord, Chunk>) -> HashMap<Region, Vec<(ChunkCoord, &Chunk)>> {
	let mut regions: HashMap<Region, Vec<(ChunkCoord, &Chunk)>> = HashMap::new();
	
	for (coord, chunk) in chunks {
		let region_coord = Region::from_chunk_coord(*coord);
		regions.entry(region_coord).or_default().push((*coord, chunk));
	}
	
	regions
}


/// Loads the entire world from disk
pub fn load_entire_world(world_path: &Path) -> Result<()> {
	let region_dir = world_path.join("region");
	
	let entries = fs::read_dir(region_dir).map_err(|e| {
		if e.kind() == ErrorKind::NotFound {
			Error::new(ErrorKind::NotFound, "World directory not found")
		} else {
			e
		}
	})?;
	
	let mut loaded_world = World::empty();
	
	for entry in entries {
		let entry = entry?;
		let path = entry.path();
		
		if !path.is_file() || !is_valid_region_filename(&path) {
			continue;
		}
		
		if let Err(e) = load_region_file(&path, &mut loaded_world) {
			println!("Failed to load region file {:?}: {}", path, e);
		}
	}
	
	let game_world = ptr::get_gamestate().world_mut();
	
	// Transfer loaded chunks
	for (coord, chunk) in loaded_world.chunks {
		game_world.chunks.insert(coord, chunk);
		game_world.loaded_chunks.insert(coord);
		game_world.create_bind_group(coord); // might want to create this in off-thread or atleast not at once...
	}
	Ok(())
}

/// Generates the path for a region file
fn region_file_path(region_dir: &Path, coord: Region) -> PathBuf {
	let (x, y, z) = coord.unpack();
	let filename = format!("{}{}.{}.{}{}", Region::PREFIX, x, y, z, Region::SUFFIX);
	region_dir.join(filename)
}

/// Checks if a filename matches the region file pattern
fn is_valid_region_filename(path: &Path) -> bool {
	path.file_name()
		.and_then(|n| n.to_str())
		.map(|name| name.starts_with(Region::PREFIX) && name.ends_with(Region::SUFFIX))
		.unwrap_or(false)
}

/// Writes data to a file atomically using a temporary file
fn write_atomic(temp_path: &Path, final_path: &Path, data: &[u8]) -> Result<()> {
	// Write to temp file
	{
		let mut file = File::create(temp_path)?;
		file.write_all(data)?;
		file.sync_all()?; // Ensure data is flushed to disk
	}
	
	// Atomic rename
	fs::rename(temp_path, final_path)?;
	Ok(())
}

/// Loads a single region file into the world
fn load_region_file(path: &Path, world: &mut World) -> Result<()> {
	let region_coord = parse_region_filename(path)?;
	let bytes = fs::read(path)?;
	
	if bytes.len() < 4 {
		return Err(Error::new(ErrorKind::InvalidData, "Region file too small"));
	}
	
	load_chunks_from_bytes(&bytes, region_coord, world)
}

/// Parses region coordinates from filename
fn parse_region_filename(path: &Path) -> Result<Region> {
	let filename = path.file_name().and_then(|n| n.to_str())
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid filename"))?;
	
	let coords_str = filename
		.strip_prefix(Region::PREFIX).and_then(|s| s.strip_suffix(Region::SUFFIX))
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid region filename"))?;
	
	let parts: Vec<&str> = coords_str.split('.').collect();
	if parts.len() != 3 {
		return Err(Error::new(ErrorKind::InvalidData, "Invalid coordinate format"));
	}
	
	let parse_coord = |s: &str| -> Result<i32> {
		s.parse().map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid coordinate value"))
	};
	
	Ok(Region::new(
		parse_coord(parts[0])?,
		parse_coord(parts[1])?,
		parse_coord(parts[2])?,
	))
}

/// Loads chunks from binary region data
fn load_chunks_from_bytes(bytes: &[u8], region_coord: Region, world: &mut World) -> Result<()> {
	if bytes.len() < 8 {
		return Err(Error::new(ErrorKind::InvalidData, "Region file too small"));
	}
	
	// Read chunk count
	let chunk_count = usize::from_binary(&bytes[0..8])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk count"))?;
	let mut cursor = 8;
	
	// Read each chunk iteratively
	for _ in 0..chunk_count {
		if cursor >= bytes.len() {
			break; // Prevent out-of-bounds access
		}
		
		match load_chunk(bytes, &mut cursor, region_coord) {
			Ok((coord, chunk)) => {
				world.chunks.insert(coord, chunk);
			}
			Err(e) => {
				println!("Failed to load chunk at cursor {}: {}", cursor, e);
				break;
			}
		}
	}
	
	Ok(())
}

/// Loads a single chunk from binary data
fn load_chunk(bytes: &[u8], cursor: &mut usize, region_coord: Region) -> Result<(ChunkCoord, Chunk)> {
	// Read local coordinates
	if *cursor + 2 > bytes.len() {
		return Err(Error::new(ErrorKind::InvalidData, "Insufficient data for coordinates"));
	}
	let local_coord = LocalPos::from_binary(&bytes[*cursor..*cursor + 2]).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid coordinate data"))?;
	*cursor += 2;
	let global_coord = region_coord.to_chunk_coord() + local_coord.to_chunk_coord();
	
	// Read chunk data
	let mut chunk = Chunk::from_binary(&bytes[*cursor..]).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk data"))?;
	
	*cursor += chunk.binary_size();
	
	// Handle compression if needed
	if let Some(storage) = BlockStorage::from_rle(&chunk.storage()) {
		*chunk.storage_mut() = storage;
	}
	
	Ok((global_coord, chunk))
}
