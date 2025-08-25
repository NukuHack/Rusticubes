
use crate::{
	block::{
		main::Chunk,
		math::{ChunkCoord, LocalPos, REGION_SIZE_U, SUFFIX, PREFIX},
		storage::BlockStorage,
	},
	ext::ptr,
	fs::binary::{BinarySerializable, FixedBinarySerializable},
	utils::time::Time,
	world::main::World,
};
use std::{
	collections::HashMap,
	fs::{self, File},
	hash::BuildHasherDefault,
	io::{Error, ErrorKind, Read, Result, Write},
	path::{Path, PathBuf},
	sync::mpsc,
};
use ahash::AHasher;
use crossbeam::thread;

// Type aliases
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

// Constants
const TEMP_FILE_SUFFIX: &str = ".tmp";
const MAX_SAVE_THREADS: usize = 4;

// Save operation tracking
#[derive(Clone)]
struct SaveOperation {
	region_coord: ChunkCoord,
	chunk_data: Vec<(ChunkCoord, Vec<u8>)>,
	file_path: PathBuf,
}

// Directory management
// ===================

#[inline]
pub fn get_save_path() -> PathBuf {
	let base_path = if cfg!(windows) {
		dirs::document_dir().unwrap_or_else(|| PathBuf::from(".")).join("My Games")
	} else if cfg!(target_os = "macos") {
		dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Library/Application Support")
	} else {
		dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Games")
	};

	base_path.join("Rusticubes")
}

#[inline]
pub fn ensure_save_dir() -> Result<PathBuf> {
	let path = get_save_path();
	fs::create_dir_all(&path)?;
	Ok(path)
}

pub fn get_world_names() -> Result<Vec<String>> {
	let saves_path = get_save_path().join("saves");
	
	let world_names = fs::read_dir(saves_path)?
		.filter_map(|entry| {
			let entry = entry.ok()?;
			let path = entry.path();
			
			if path.is_dir() {
				path.file_name()?.to_str().map(|s| s.to_string())
			} else {
				None
			}
		})
		.collect();
	
	Ok(world_names)
}

pub fn del_world(world_name: &str) -> Result<()> {
	let saves_path = get_save_path().join("saves").canonicalize()
		.map_err(|e| Error::new(ErrorKind::NotFound, format!("Failed to access saves directory: {}", e)))?;

	let target_path = saves_path.join(world_name);
	
	if !target_path.exists() {
		return Err(Error::new(ErrorKind::NotFound, format!("World '{}' does not exist", world_name)));
	}
	
	if !target_path.is_dir() {
		return Err(Error::new(ErrorKind::InvalidInput, format!("'{}' is not a directory", world_name)));
	}

	fs::remove_dir_all(&target_path)?;
	
	// Only update UI on success
	let state = ptr::get_state();
	state.ui_manager.setup_ui();
	
	Ok(())
}

// World Data Management
// =====================

#[derive(Debug)]
pub struct WorldData {
	pub version: String,
	pub creation_date: Time,
	pub last_opened_date: Time,
}

impl WorldData {
	pub fn new() -> Self {
		Self {
			version: env!("CARGO_PKG_VERSION").to_string(),
			creation_date: Time::now(),
			last_opened_date: Time::now(),
		}
	}

	pub fn update_last_opened(&mut self) {
		self.last_opened_date = Time::now();
	}
}

impl BinarySerializable for WorldData {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(self.binary_size());
		data.extend_from_slice(&self.version.to_binary());
		data.extend_from_slice(&self.creation_date.to_binary());
		data.extend_from_slice(&self.last_opened_date.to_binary());
		data
	}

	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let mut offset = 0;
		
		let version = String::from_binary(&bytes[offset..])?;
		offset += version.binary_size();
		
		if bytes.len() < offset + Time::BINARY_SIZE * 2 {
			return None;
		}
		
		let creation_date = Time::from_binary(&bytes[offset..offset + Time::BINARY_SIZE])?;
		offset += Time::BINARY_SIZE;
		
		let last_opened_date = Time::from_binary(&bytes[offset..offset + Time::BINARY_SIZE])?;
		
		Some(Self {
			version,
			creation_date,
			last_opened_date,
		})
	}

	fn binary_size(&self) -> usize {
		self.version.binary_size() + Time::BINARY_SIZE * 2
	}
}

pub fn load_world_data(path: &Path) -> Result<WorldData> {
	let file_path = path.join("world_data.dat");
	
	match File::open(&file_path) {
		Ok(mut file) => {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes)?;
			WorldData::from_binary(&bytes)
				.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid world data"))
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
	if let Some(parent) = file_path.parent() {
		fs::create_dir_all(parent)?;
	}
	
	let temp_path = file_path.with_extension(TEMP_FILE_SUFFIX);
	let bytes = data.to_binary();
	
	{
		let mut file = File::create(&temp_path)?;
		file.write_all(&bytes)?;
	}
	
	fs::rename(temp_path, file_path)?;
	Ok(())
}

pub fn update_world_data(path: &Path) -> Result<WorldData> {
	let mut world_data = load_world_data(path)?;
	let current_version = env!("CARGO_PKG_VERSION");
	
	if world_data.version != current_version {
		world_data.version = current_version.to_string();
	}
	
	world_data.update_last_opened();
	save_world_data(path, &world_data)?;
	
	Ok(world_data)
}

// World Save/Load Operations
// ==========================

/// Saves the entire world to disk using multi-threaded region processing
pub fn save_entire_world(world_path: &Path) -> Result<()> {
	let game_state = ptr::get_gamestate();
	let world = game_state.world();
	
	if world.chunks.is_empty() {
		return Ok(());
	}
	
	let regions = group_chunks_by_region(&world.chunks);
	let region_dir = world_path.join("region");
	fs::create_dir_all(&region_dir)?;
	
	let save_operations = prepare_save_operations(regions, &region_dir);
	
	if save_operations.is_empty() {
		return Ok(());
	}
	
	execute_save_operations(save_operations)?;
	
	Ok(())
}

fn prepare_save_operations(
	regions: HashMap<ChunkCoord, Vec<(ChunkCoord, &Chunk)>>,
	region_dir: &Path,
) -> Vec<SaveOperation> {
	let mut operations = Vec::new();
	
	for (region_coord, chunks) in regions {
		let chunk_data: Vec<_> = chunks
			.into_iter()
			.filter(|(_, chunk)| chunk.finished_gen())
			.map(|(coord, chunk)| (coord, chunk.to_binary()))
			.collect();
		
		if !chunk_data.is_empty() {
			operations.push(SaveOperation {
				region_coord,
				chunk_data,
				file_path: region_file_path(region_dir, region_coord),
			});
		}
	}
	
	operations
}

// Simplify save operations by removing unnecessary cloning
fn execute_save_operations(operations: Vec<SaveOperation>) -> Result<()> {
	let errors = thread::scope(|s| {
		let (sender, receiver) = mpsc::channel();
		let num_workers = operations.len().min(MAX_SAVE_THREADS);
		let operations_per_worker = (operations.len() + num_workers - 1) / num_workers;
		
		for chunk in operations.chunks(operations_per_worker) {
			let sender = sender.clone();
			let operations = chunk.to_vec();
			
			s.spawn(move |_| {
				for operation in operations {
					if let Err(e) = save_region_threaded(operation) {
						let _ = sender.send(e);
					}
				}
			});
		}
		
		drop(sender);
		
		// Collect all errors instead of just printing them
		receiver.iter().collect::<Vec<_>>()
	}).unwrap_or_else(|_| {
		vec![Error::new(ErrorKind::Other, "Thread panic during save operation")]
	});
	
	// Return first error if any occurred
	if let Some(first_error) = errors.into_iter().next() {
		return Err(first_error);
	}
	
	Ok(())
}

fn validate_save_operation(operation: &SaveOperation) -> Result<()> {
	if operation.chunk_data.is_empty() {
		return Err(Error::new(ErrorKind::InvalidInput, "No chunks to save"));
	}
	
	// Validate that all chunks belong to the region
	for (coord, _) in &operation.chunk_data {
		let local = coord_to_local(*coord, operation.region_coord);
		if local.0 >= REGION_SIZE_U || local.1 >= REGION_SIZE_U || local.2 >= REGION_SIZE_U {
			return Err(Error::new(ErrorKind::InvalidData, 
				format!("Chunk {:?} does not belong to region {:?}", coord, operation.region_coord)));
		}
	}
	
	Ok(())
}

fn save_region_threaded(operation: SaveOperation) -> Result<()> {
	validate_save_operation(&operation)?;
	
	let SaveOperation {
		region_coord,
		chunk_data,
		file_path,
	} = operation;
	
	let temp_path = file_path.with_extension(TEMP_FILE_SUFFIX);
	let existing_chunks = load_existing_chunks(&file_path, region_coord).unwrap_or_default();
	
	let mut merged: HashMap<ChunkCoord, Vec<u8>> = existing_chunks
		.into_iter()
		.map(|(coord, chunk)| (coord, chunk.to_binary()))
		.collect();
	
	for (coord, data) in chunk_data {
		merged.insert(coord, data);
	}
	
	let final_chunks: Vec<_> = merged.into_iter().collect();
	let data = serialize_region_data(&final_chunks, region_coord)?;
	
	write_atomic(&temp_path, &file_path, &data)?;
	Ok(())
}

fn serialize_region_data(chunks: &[(ChunkCoord, Vec<u8>)], region_coord: ChunkCoord) -> Result<Vec<u8>> {
	let mut data = Vec::with_capacity(1024 * chunks.len());
	data.extend_from_slice(&chunks.len().to_binary());
	
	for (coord, chunk_data) in chunks {
		let local_coord = coord_to_local(*coord, region_coord);
		let packed_coord = LocalPos::from(local_coord);
		data.extend_from_slice(&packed_coord.to_binary());
		data.extend_from_slice(chunk_data);
	}
	
	Ok(data)
}

fn coord_to_local(global_coord: ChunkCoord, region_coord: ChunkCoord) -> (u8, u8, u8) {
	let region_origin = region_coord.from_region_step();
	(
		(global_coord.x() - region_origin.x()) as u8,
		(global_coord.y() - region_origin.y()) as u8,
		(global_coord.z() - region_origin.z()) as u8,
	)
}

fn group_chunks_by_region(chunks: &FastMap<ChunkCoord, Chunk>) -> HashMap<ChunkCoord, Vec<(ChunkCoord, &Chunk)>> {
	let mut regions: HashMap<ChunkCoord, Vec<(ChunkCoord, &Chunk)>> = HashMap::new();
	
	for (coord, chunk) in chunks {
		let region_coord = ChunkCoord::to_region_step(*coord);
		regions.entry(region_coord).or_default().push((*coord, chunk));
	}
	
	regions
}

fn load_existing_chunks(path: &Path, region_coord: ChunkCoord) -> Result<Vec<(ChunkCoord, Chunk)>> {
	if !path.exists() {
		return Ok(Vec::new());
	}
	
	let bytes = fs::read(path)?;
	let mut chunks = Vec::new();
	
	if bytes.len() < 8 {
		return Ok(chunks);
	}
	
	let chunk_count = usize::from_binary(&bytes[0..8])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk count"))?;
	let mut cursor = 8;
	
	for i in 0..chunk_count {
		if cursor >= bytes.len() {
			return Err(Error::new(ErrorKind::InvalidData, 
				format!("Unexpected end of file at chunk {}/{}", i + 1, chunk_count)));
		}
		
		let (coord, chunk) = load_chunk(&bytes, &mut cursor, region_coord)
			.map_err(|e| Error::new(ErrorKind::InvalidData, 
				format!("Failed to load chunk {}/{}: {}", i + 1, chunk_count, e)))?;
		chunks.push((coord, chunk));
	}
	
	Ok(chunks)
}

// World Loading
// =============

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
	
	for (coord, chunk) in loaded_world.chunks {
		game_world.chunks.insert(coord, chunk);
		game_world.loaded_chunks.insert(coord);
		game_world.create_bind_group(coord);
	}
	
	Ok(())
}

fn load_region_file(path: &Path, world: &mut World) -> Result<()> {
	let region_coord = parse_region_filename(path)?;
	let bytes = fs::read(path)?;

	if bytes.len() < usize::BINARY_SIZE {
		return Err(Error::new(ErrorKind::InvalidData, "ChunkCoord file too small"));
	}
	
	let chunk_count = usize::from_binary(&bytes[0..usize::BINARY_SIZE])
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid chunk count"))?;
	let mut cursor = usize::BINARY_SIZE;
	
	for i in 0..chunk_count {
		if cursor >= bytes.len() {
			return Err(Error::new(ErrorKind::InvalidData, 
				format!("Unexpected end of file at chunk {}/{}", i + 1, chunk_count)));
		}
		
		let (coord, chunk) = load_chunk(&bytes, &mut cursor, region_coord)
			.map_err(|e| Error::new(ErrorKind::InvalidData, 
				format!("Failed to load chunk {}/{}: {}", i + 1, chunk_count, e)))?;
		
		world.chunks.insert(coord, chunk);
	}
	
	Ok(())
}

fn load_chunk(bytes: &[u8], cursor: &mut usize, region_coord: ChunkCoord) -> Result<(ChunkCoord, Chunk)> {
	if *cursor + 2 > bytes.len() {
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
	
	if let Some(storage) = BlockStorage::from_rle(&chunk.storage()) {
		*chunk.storage_mut() = storage;
	}
	
	Ok((global_coord, chunk))
}

// Utility Functions
// =================

fn region_file_path(region_dir: &Path, coord: ChunkCoord) -> PathBuf {
	let (x, y, z) = coord.unpack();
	let filename = format!("{}{}.{}.{}{}", PREFIX, x, y, z, SUFFIX);
	region_dir.join(filename)
}

fn is_valid_region_filename(path: &Path) -> bool {
	path.file_name()
		.and_then(|n| n.to_str())
		.map(|name| name.starts_with(PREFIX) && name.ends_with(SUFFIX))
		.unwrap_or(false)
}

fn write_atomic(temp_path: &Path, final_path: &Path, data: &[u8]) -> Result<()> {
	{
		let mut file = File::create(temp_path)?;
		file.write_all(data)?;
		file.sync_all()?;
	}
	
	fs::rename(temp_path, final_path)?;
	Ok(())
}

fn parse_region_filename(path: &Path) -> Result<ChunkCoord> {
	let filename = path.file_name()
		.and_then(|n| n.to_str())
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid filename"))?;
	
	let coords_str = filename
		.strip_prefix(PREFIX)
		.and_then(|s| s.strip_suffix(SUFFIX))
		.ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid region filename"))?;
	
	let parts: Vec<&str> = coords_str.split('.').collect();
	if parts.len() != 3 {
		return Err(Error::new(ErrorKind::InvalidData, "Invalid coordinate format"));
	}
	
	let parse_coord = |s: &str| s.parse()
		.map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid coordinate value"));
	
	Ok(ChunkCoord::new(
		parse_coord(parts[0])?,
		parse_coord(parts[1])?,
		parse_coord(parts[2])?,
	))
}
