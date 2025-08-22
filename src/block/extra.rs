
use crate::player::Player;
use crate::ext::ptr;
use crate::block::math::ChunkCoord;
use crate::game::player::Camera;
use crate::block::main::Chunk;
use crate::world::main::World;
use glam::{Vec3, IVec3};

pub const REACH: f32 = 8.;
pub const RENDER_DISTANCE: f32 = 120.;

/// Helper function to update a chunk mesh after modification
#[inline]
pub fn update_chunk_mesh(world: &mut World, chunk_coord: ChunkCoord) {
	
	// Get raw pointer to the world's chunks
	let world_ptr = world as *mut World;
	
	// Get mutable reference to our chunk
	let Some(chunk) = world.get_chunk_mut(chunk_coord) else { return; };

	let state = ptr::get_state();
	
	// SAFETY:
	// 1. We only use the pointer to access different chunks than the one we're modifying
	// 2. The references don't outlive this scope
	// 3. We don't modify through these references
	let neighbors = unsafe {
		let world_ref = &*world_ptr;
		world_ref.get_neighboring_chunks(chunk_coord)
	};

	chunk.make_mesh(
		state.device(),
		state.queue(),
		neighbors);

	world.set_adjacent_un_final(chunk_coord);
}

/// Improved raycasting function that finds the first non-empty block and its face
/// Optimized raycasting function using IVec3 for block positions
#[inline]
pub fn raycast_to_block(camera: &Camera, player: &Player, world: &World, max_distance: f32) -> Option<(IVec3, IVec3)> {
	let ray_origin = player.cam_pos();
	let ray_dir = camera.forward();
	
	// Initialize variables for DDA algorithm
	let step = Vec3::new(ray_dir.x.signum(), ray_dir.y.signum(), ray_dir.z.signum());
	let step_i = IVec3::new(step.x as i32, step.y as i32, step.z as i32);
	
	// Use IVec3 for block position
	let mut block_pos = IVec3::new(
		ray_origin.x.floor() as i32,
		ray_origin.y.floor() as i32,
		ray_origin.z.floor() as i32,
	);
	
	let t_delta = Vec3::new(
		1.0 / ray_dir.x.abs().max(f32::MIN_POSITIVE),
		1.0 / ray_dir.y.abs().max(f32::MIN_POSITIVE),
		1.0 / ray_dir.z.abs().max(f32::MIN_POSITIVE),
	);
	
	let mut t_max = Vec3::new(
		if step.x > 0.0 {
			(block_pos.x + 1) as f32 - ray_origin.x
		} else {
			ray_origin.x - block_pos.x as f32
		} / ray_dir.x.abs().max(f32::MIN_POSITIVE),
		if step.y > 0.0 {
			(block_pos.y + 1) as f32 - ray_origin.y
		} else {
			ray_origin.y - block_pos.y as f32
		} / ray_dir.y.abs().max(f32::MIN_POSITIVE),
		if step.z > 0.0 {
			(block_pos.z + 1) as f32 - ray_origin.z
		} else {
			ray_origin.z - block_pos.z as f32
		} / ray_dir.z.abs().max(f32::MIN_POSITIVE),
	);
	
	let mut normal = IVec3::ZERO;
	let mut traveled = 0.0f32;
	
	while traveled < max_distance {
		// Check current block - now using IVec3
		if !world.get_block(block_pos).is_empty() {
			return Some((block_pos, normal));
		}
		
		// Move to next block boundary
		if t_max.x < t_max.y && t_max.x < t_max.z {
			normal = IVec3::new(-step_i.x, 0, 0);
			block_pos.x += step_i.x;
			traveled = t_max.x;
			t_max.x += t_delta.x;
		} else if t_max.y < t_max.z {
			normal = IVec3::new(0, -step_i.y, 0);
			block_pos.y += step_i.y;
			traveled = t_max.y;
			t_max.y += t_delta.y;
		} else {
			normal = IVec3::new(0, 0, -step_i.z);
			block_pos.z += step_i.z;
			traveled = t_max.z;
			t_max.z += t_delta.z;
		}
	}
	
	None
}

pub fn get_block_id_from_item_name(item_name: &str) -> u16 {
	use crate::render::texture::get_texture_map;
	get_texture_map().iter()
		.position(|name| name == item_name)
		.map(|idx| idx as u16)
		.unwrap_or(0)
}

pub fn get_item_name_from_block_id(block_id: u16) -> String {
	use crate::render::texture::get_texture_map;
	get_texture_map()
		.get(block_id as usize)
		.map(|s| s.to_string())
		.unwrap_or("0".to_string())
}


/// Loads a chunk at the camera's position if not already loaded
#[inline]
pub fn add_full_chunk() {
	let state = ptr::get_state();
	if !state.is_world_running { return; }
	
	let pos:Vec3 = ptr::get_gamestate().player().pos();
	let chunk_coord = ChunkCoord::from_world_posf(pos);

	let world = ptr::get_gamestate().world_mut();
	world.set_chunk(chunk_coord, Chunk::empty());
	world.create_bind_group(chunk_coord);
	update_chunk_mesh(world, chunk_coord);
}

/// Loads chunks around the camera in a radius
#[inline]
pub fn update_full_world() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	ptr::get_gamestate().world_mut().update_loaded_chunks(
		ptr::get_gamestate().player().pos(),
		REACH * 2.0,
	);

	let state_b = ptr::get_state();
	ptr::get_gamestate()
		.world_mut()
		.make_chunk_meshes(state_b.device(), state_b.queue());
}

/// Fill chunks around the camera in a radius
#[inline]
pub fn add_full_world() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	
	let seed = ptr::get_gamestate().world().seed();
	let thread_count = ptr::get_gamestate().world().thread_count();
	*ptr::get_gamestate().world_mut() = World::empty();
	ptr::get_gamestate().world_mut().set_seed(seed);
	ptr::get_gamestate().world_mut().set_thread_count(thread_count);
	ptr::get_gamestate().world_mut().start_generation_threads(thread_count);

	let state_b = ptr::get_state();
	ptr::get_gamestate()
		.world_mut()
		.make_chunk_meshes(state_b.device(), state_b.queue());
}
