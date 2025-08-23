
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
	
	// Save each region with error handling
	for (region_coord, chunks) in regions {
		if let Err(e) = save_region(&region_dir, region_coord, chunks) {
			eprintln!("Failed to save region {:?}: {}", region_coord, e);
			// Continue saving other regions
		}
	}
	
	Ok(())
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
			eprintln!("Failed to load region file {:?}: {}", path, e);
		}
	}
	
	finalize_world_loading(loaded_world);
	Ok(())
}

/// Groups chunks by their containing region coordinates
fn group_chunks_by_region(chunks: &FastMap<ChunkCoord, Chunk>) -> HashMap<Region, Vec<(ChunkCoord, Chunk)>> {
	let mut regions: HashMap<Region, Vec<(ChunkCoord, Chunk)>> = HashMap::new();
	
	for (&coord, chunk) in chunks {
		let region_coord = Region::from_chunk_coord(coord);
		regions.entry(region_coord)
			.or_default()
			.push((coord, chunk.clone()));
	}
	
	regions
}

/// Saves a single region file
fn save_region(region_dir: &Path, region_coord: Region, chunks: Vec<(ChunkCoord, Chunk)>) -> Result<()> {
	let file_path = region_file_path(region_dir, region_coord);
	let temp_path = file_path.with_extension(TEMP_FILE_SUFFIX);
	
	// Merge with existing chunks if file exists
	let final_chunks = merge_with_existing_chunks(&file_path, region_coord, chunks)?;
	
	// Serialize and write atomically
	let data = serialize_region(&final_chunks, region_coord)?;
	write_atomic(&temp_path, &file_path, &data)?;
	
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

/// Serializes region data to binary format
fn serialize_region(chunks: &[(ChunkCoord, Chunk)], region_coord: Region) -> Result<Vec<u8>> {
	let mut data = Vec::with_capacity(1024); // Initial capacity
	
	// Write chunk count
	data.extend_from_slice(&chunks.len().to_binary());
	
	// Write each chunk
	for (coord, chunk) in chunks {
		let local_coord = coord_to_local(*coord, region_coord);
		let packed_coord = LocalPos::from(local_coord);
		data.extend_from_slice(&packed_coord.to_binary());
		data.extend_from_slice(&chunk.to_binary());
	}
	
	Ok(data)
}

/// Converts global chunk coordinate to local coordinate within region
fn coord_to_local(global_coord: ChunkCoord, region_coord: Region) -> (u8,u8,u8) {
	let region_origin = region_coord.to_chunk_coord();
	(
		(global_coord.x() - region_origin.x()) as u8,
		(global_coord.y() - region_origin.y()) as u8,
		(global_coord.z() - region_origin.z()) as u8,
	)
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
	
	// Read chunk count
	let chunk_count = usize::from_binary(&bytes).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid usize data"))?;
	let mut cursor = 8;
	
	// Read each chunk
	for _ in 0..chunk_count {
		match load_chunk(bytes, &mut cursor, region_coord) {
			Ok((coord, chunk)) => {
				world.chunks.insert(coord, chunk);
			}
			Err(e) => {
				eprintln!("Failed to load chunk: {}", e);
				break;
			}
		}
	}
	
	Ok(())
}

/// Loads a single chunk from binary data
fn load_chunk(bytes: &[u8], cursor: &mut usize, region_coord: Region) -> Result<(ChunkCoord, Chunk)> {
	// Read local coordinates
	let local_coord = read_local_coord(bytes, cursor)?;
	let global_coord = region_coord.to_chunk_coord() + local_coord.to_chunk_coord();
	
	// Read chunk data
	let chunk = Chunk::from_binary(&bytes[*cursor..]).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk data"))?;
	
	*cursor += chunk.binary_size();
	
	// Handle compression if needed
	let chunk = maybe_decompress_chunk(chunk)?;
	
	Ok((global_coord, chunk))
}

/// Reads and unpacks local coordinates
fn read_local_coord(bytes: &[u8], cursor: &mut usize) -> Result<LocalPos> {
	if *cursor + 2 > bytes.len() {
		return Err(Error::new(ErrorKind::InvalidData, "Insufficient data for coordinates"));
	}
	
	let pos = LocalPos::from_binary(&bytes[*cursor..*cursor + 2]).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid coordinate data"))?;
	*cursor += 2;
	
	Ok(pos)
}

/// Decompresses chunk storage if needed
fn maybe_decompress_chunk(mut chunk: Chunk) -> Result<Chunk> {
	if let Some(storage) = BlockStorage::from_rle(&chunk.storage()) {
		*chunk.storage_mut() = storage;
	}
	Ok(chunk)
}

/// Finalizes world loading process
fn finalize_world_loading(world: World) {
	#[allow(unused_mut)]
	let mut game_world = ptr::get_gamestate().world_mut();
	
	// Transfer loaded chunks
	for (coord, chunk) in world.chunks {
		game_world.chunks.insert(coord, chunk);
		game_world.loaded_chunks.insert(coord);
		game_world.create_bind_group(coord); // might want to create this in off-thread or atleast not at once...
	}
}

/// Merges new chunks with existing chunks from disk
fn merge_with_existing_chunks(path: &Path, region_coord: Region, new_chunks: Vec<(ChunkCoord, Chunk)>) -> Result<Vec<(ChunkCoord, Chunk)>> {
	if !path.exists() {
		return Ok(new_chunks);
	}
	
	let existing_chunks = match load_existing_chunks(path, region_coord) {
		Ok(chunks) => chunks,
		Err(e) => {
			eprintln!("Failed to load existing chunks: {}", e);
			return Ok(new_chunks);
		}
	};
	
	// Use a map to handle duplicates (new chunks take precedence)
	let mut merged = HashMap::new();
	for (coord, chunk) in existing_chunks {
		merged.insert(coord, chunk);
	}
	for (coord, chunk) in new_chunks {
		if !chunk.finished_gen() { continue } // only save chunks that are sure to finished generating
		merged.insert(coord, chunk);
	}
	
	Ok(merged.into_iter().collect())
}

/// Loads existing chunks from a region file
fn load_existing_chunks(path: &Path, region_coord: Region) -> Result<Vec<(ChunkCoord, Chunk)>> {
	let bytes = fs::read(path)?;
	let mut chunks = Vec::new();
	
	let chunk_count = usize::from_binary(&bytes).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid usize data"))?;
	let mut cursor = 8;
	
	for _ in 0..chunk_count {
		if let Ok((coord, chunk)) = load_chunk(&bytes, &mut cursor, region_coord) {
			chunks.push((coord, chunk));
		} else {
			break;
		}
	}
	
	Ok(chunks)
}
