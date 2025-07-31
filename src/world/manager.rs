
use crate::world::main::World;
use crate::ext::ptr;
use crate::hs::binary::BinarySerializable;
use crate::hs::time;
use std::path::{Path, PathBuf};
use std::io::{Read, Write, Result, Error, ErrorKind};
use std::fs::{self, File};

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


pub fn load_world_data(path: &Path) -> Result<WorldData> {
	let file_path = path.join("world_data.dat");
	match File::open(&file_path) {
		Ok(mut file) => {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes)?;
			WorldData::from_binary(&bytes).ok_or_else(|| {
				std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid world data")
			})
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





// 
// Main world Save - Load functions
// will have to rework them a bit, because the giant world size
// currently a normal world is around 100Kb so it's fine for a single function
// but theoretically it can get up to Billions of Tb, not like anyone will ever go that far
// so yeah processing big worlds with a single function is bad
// 

pub fn save_entire_world(path: &PathBuf) -> Result<()> {
	let game_state = ptr::get_gamestate();
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
	let mut loaded_world = World::from_binary(&bytes).ok_or_else(|| {
		Error::new(ErrorKind::InvalidData, "Failed to deserialize world")
	})?;
	// Apply the loaded world
	for (chunk_coord, _chunk) in loaded_world.chunks.clone().iter() {
		loaded_world.loaded_chunks.insert(*chunk_coord);
		loaded_world.create_bind_group(*chunk_coord);
	}
	*ptr::get_gamestate().world_mut() = loaded_world;
		
	Ok(())
}
