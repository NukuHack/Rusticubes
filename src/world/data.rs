

use crate::{
	fs::binary::{BinarySerializable, FixedBinarySerializable},
	utils::time::Time,
};
use std::{
	fs::{self, File},
	io::{Error, ErrorKind, Read, Result, Write},
	path::Path,
};
use crate::world::manager::TEMP_FILE_SUFFIX;


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
