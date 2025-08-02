
use crate::ext::config::CameraConfig;
use crate::world::manager::get_save_path;
use crate::world::manager::ensure_save_dir;
use crate::game::player;
use crate::ext::ptr;
use crate::world;
#[cfg(not(test))]
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use glam::Vec3;

#[allow(dead_code)]
pub struct GameState {
	worldname: String,
	player: player::Player,
	world: world::main::World, // lol main data storage :)
	save_path: std::path::PathBuf,
	world_seed: u32,
	is_running: bool,
}

pub fn make_world(save_path: PathBuf) {
	// has to make the error handling better , make the error quit from world
	let _ = ensure_save_dir();
	// Check and create directories if needed
	match std::fs::metadata(&save_path) {
		Ok(metadata) => {
			if !metadata.is_dir() {
				println!(
					"Save path {:?} exists but is not a directory", 
					save_path
				);
			}
			// Directory already exists, no need to create
		}
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
			// Directory doesn't exist, try to create it
			std::fs::create_dir_all(&save_path)
				.map_err(|e| {
					println!("Failed to create save directory at {:?}: {}", save_path, e)
				}).unwrap_or_else(|_| println!("Something failed"));
		}
		Err(e) => {
			// Other IO error (permission issues, etc.)
			println!(
				"Unexpected error accessing save path {:?}: {}", 
				save_path, e
			);
		}
	}
}

#[allow(dead_code)]
impl GameState {
	#[inline] pub fn new(worldname: &str) -> Self {
		#[cfg(test)]
		let player = {
			let offset = Vec3::new(0., 1.7, 0.);
			let pos = Vec3::new(0.5, 0.5, 0.5);
			player::Player::dummy(pos, CameraConfig::new(offset))
		};
		#[cfg(not(test))]
		let player = {
			use crate::item::items::{ItemStack, ItemId};
			use crate::item::inventory::AreaType;
			let state = ptr::get_state();
			let offset = Vec3::new(0., 1.7, 0.);
			let pos = Vec3::new(0.5, 0.5, 0.5);
			let mut player = player::Player::new(
				CameraConfig::new(offset),
				pos,
				state.device(),
				*state.size(),
				&state.render_context.layouts[1],
			);
			player
				.inventory_mut()
				.get_area_mut(AreaType::Hotbar)
				.add_item(ItemStack::new_i(ItemId::from_str("brick_grey")).with_stack(5));
			player
		};
		
		// Create the save path
		let save_path = get_save_path()
			.join("saves")
			.join(worldname);

		make_world(save_path.clone());

		let creation_date:u64 = match world::manager::update_world_data(&save_path) {
			Ok(data) => data.creation_date.to_unix_timestamp(),
			Err(e) => {
				println!("Error updating world data: {}", e);
				0
			},
		};
		// Combine worldname and creation_date into a seed
		let world_seed = {
			// Simple but effective hash function
			let mut hash: u32 = 0;
			for (i, c) in worldname.chars().enumerate() {
				hash = hash.wrapping_add(c as u32)
						  .wrapping_mul(i as u32 + 1)
						  .wrapping_add((creation_date << 32 ) as u32)
						  .rotate_left(3);
			}
			hash.wrapping_add(creation_date as u32)
		};
		Self {
			worldname: worldname.to_string(),
			player,
			world: world::main::World::empty(),
			save_path,
			world_seed,
			is_running: false,
		}
	}
	#[inline] pub const fn world_mut(&mut self) -> &mut world::main::World {
		&mut self.world
	}
	#[inline] pub const fn player_mut(&mut self) -> &mut player::Player {
		&mut self.player
	}
	#[inline] pub const fn worldname(&self) -> &String {
		&self.worldname
	}
	#[inline] pub const fn player(&self) -> &player::Player {
		&self.player
	}
	#[inline] pub const fn world(&self) -> &world::main::World {
		&self.world
	}
	#[inline] pub const fn save_path(&self) -> &std::path::PathBuf {
		&self.save_path
	}
	#[inline] pub const fn is_running(&self) -> bool {
		self.is_running
	}
	#[inline] pub const fn running(&mut self) -> &mut bool {
		&mut self.is_running
	}
	#[inline] pub const fn seed(&self) -> &u32 {
		&self.world_seed
	}
}

#[inline]
pub fn start_world(worldname: &str) {
	let game_state = GameState::new(worldname);
	ptr::GAMESTATE_PTR.store(Box::into_raw(Box::new(game_state)), Ordering::Release);
	
	// This will only execute when not in test configuration
	if !cfg!(test) {
		ptr::get_state().is_world_running = true;
	}
}