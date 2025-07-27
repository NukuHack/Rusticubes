
use crate::player::Player;
use crate::ext::ptr;
use crate::block::math::ChunkCoord;
use crate::game::player::Camera;
use crate::block::main::Block;
use crate::world::main::World;
use glam::{Vec3, IVec3};

const REACH: f32 = 6.0;

/// Helper function to update a chunk mesh after modification
#[inline]
fn update_chunk_mesh(world: &mut World, chunk_coord: ChunkCoord) {
	
	// Get raw pointer to the world's chunks
	let world_ptr = world as *mut World;
	
	// Get mutable reference to our chunk
	if let Some(chunk) = world.get_chunk_mut(chunk_coord) {
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

		for coord in chunk_coord.get_adjacent().iter() {
			if let Some(neighbor_chunk) = world.get_chunk_mut(*coord) {
				neighbor_chunk.final_mesh = false;
			}
		}
	}
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

/// Places a cube on the face of the block the player is looking at
#[inline]
pub fn place_looked_block() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let player = &ptr::get_gamestate().player();
	let world = &mut ptr::get_gamestate().world_mut();

	if let Some((block_pos, normal)) = raycast_to_block(player.camera(), player, world, REACH) {
		let placement_pos = block_pos + normal;
		let block_id = player.inventory()
		    .selected_item()
		    .and_then(|item| item.get_block_id())
		    .map_or(1, |block_id| block_id.inner());

		world.set_block(placement_pos, Block::new(block_id));
		update_chunk_mesh(world, ChunkCoord::from_world_pos(placement_pos));
	}
}

/// Removes the block the player is looking at
pub fn remove_targeted_block() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let world = &mut ptr::get_gamestate().world_mut();

	if let Some((block_pos, _)) = raycast_to_block(ptr::get_gamestate().player().camera(),ptr::get_gamestate().player(), world, REACH) {
		world.set_block(block_pos, Block::None);
		update_chunk_mesh(world, ChunkCoord::from_world_pos(block_pos));
	}
}


/// Loads a chunk at the camera's position if not already loaded
#[inline]
pub fn add_full_chunk() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let pos:Vec3 = ptr::get_gamestate().player().pos();
	let chunk_coord = ChunkCoord::from_world_posf(pos);

	let world = ptr::get_gamestate().world_mut();
	world.load_chunk(chunk_coord);
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
		false,
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
	ptr::get_gamestate().world_mut().update_loaded_chunks(
		ptr::get_gamestate().player().pos(),
		REACH * 2.0,
		true,
	);

	let state_b = ptr::get_state();
	ptr::get_gamestate()
		.world_mut()
		.make_chunk_meshes(state_b.device(), state_b.queue());
}
