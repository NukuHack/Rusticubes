
use crate::ext::ptr;
use crate::block::math::BlockPosition;
use crate::block::math::ChunkCoord;
use crate::game::player::Camera;
use crate::block::main::Block;
use crate::world::main::World;
use glam::{Vec3, IVec3};

const REACH: f32 = 6.0;

/// Helper function to update a chunk mesh after modification
#[inline]
fn update_chunk_mesh(world: &mut World, pos: IVec3) {
	let chunk_pos = ChunkCoord::from_world_pos(pos);
	
	// Get raw pointer to the world's chunks
	let world_ptr = world as *mut World;
	
	// Get mutable reference to our chunk
	if let Some(chunk) = world.get_chunk_mut(chunk_pos) {
		let state = ptr::get_state();
		
		// SAFETY:
		// 1. We only use the pointer to access different chunks than the one we're modifying
		// 2. The references don't outlive this scope
		// 3. We don't modify through these references
		let neighbors = unsafe {
			let world_ref = &*world_ptr;
			[
				world_ref.get_chunk(chunk_pos.offset(-1, 0, 0)),  // Left
				world_ref.get_chunk(chunk_pos.offset(1, 0, 0)), // Right
				world_ref.get_chunk(chunk_pos.offset(0, 0, -1)),   // Front
				world_ref.get_chunk(chunk_pos.offset(0, 0, 1)), // Back
				world_ref.get_chunk(chunk_pos.offset(0, 1, 0)), // Top
				world_ref.get_chunk(chunk_pos.offset(0, -1, 0)),   // Bottom
			]
		};

		chunk.make_mesh(
			state.device(),
			state.queue(),
			&neighbors,
			true,
		);
	}
}

/// Improved raycasting function that finds the first non-empty block and its face
/// Optimized raycasting function using IVec3 for block positions
#[inline]
pub fn raycast_to_block(camera: &Camera, world: &World, max_distance: f32) -> Option<(IVec3, Vec3)> {
    let ray_origin = camera.position();
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
    
    let mut normal = Vec3::ZERO;
    let mut traveled = 0.0f32;
    
    while traveled < max_distance {
        // Check current block - now using IVec3
        if !world.get_block(block_pos).is_empty() {
            return Some((block_pos, normal));
        }
        
        // Move to next block boundary
        if t_max.x < t_max.y && t_max.x < t_max.z {
            normal = Vec3::new(-step.x, 0.0, 0.0);
            block_pos.x += step_i.x;
            traveled = t_max.x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            normal = Vec3::new(0.0, -step.y, 0.0);
            block_pos.y += step_i.y;
            traveled = t_max.y;
            t_max.y += t_delta.y;
        } else {
            normal = Vec3::new(0.0, 0.0, -step.z);
            block_pos.z += step_i.z;
            traveled = t_max.z;
            t_max.z += t_delta.z;
        }
    }
    
    None
}

/// Places a cube on the face of the block the player is looking at
#[inline]
pub fn place_looked_cube() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let camera = &state.camera_system.camera();
	let world = &mut ptr::get_gamestate().world_mut();

	if let Some((block_pos, normal)) = raycast_to_block(camera, world, REACH) {
		let placement_pos = BlockPosition::from((block_pos.x as f32 + normal.x ,block_pos.y as f32 + normal.y, block_pos.z as f32 + normal.z)).into();
		world.set_block(placement_pos, Block::new(1));
		update_chunk_mesh(world, placement_pos);
	}
}

/// Removes the block the player is looking at
pub fn remove_targeted_block() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let world = &mut ptr::get_gamestate().world_mut();

	if let Some((block_pos, _)) = raycast_to_block(state.camera_system.camera(), world, REACH) {
		world.set_block(block_pos, Block::None);
		update_chunk_mesh(world, block_pos);
	}
}


/// Loads a chunk at the camera's position if not already loaded
#[inline]
pub fn add_full_chunk() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let chunk_pos = ChunkCoord::from_world_posf(state.camera_system.camera().position());

	let world = ptr::get_gamestate().world_mut();
	world.load_chunk(chunk_pos);
	world.create_bind_group(chunk_pos);
	if let Some(chunk) = ptr::get_gamestate()
		.world_mut()
		.get_chunk_mut(chunk_pos)
	{
		let state_b = ptr::get_state();
		chunk.make_mesh(state_b.device(), state_b.queue(), &world.get_neighboring_chunks(chunk_pos), true);
	}
}

/// Loads chunks around the camera in a radius
#[inline]
pub fn update_full_world() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	ptr::get_gamestate().world_mut().update_loaded_chunks(
		state.camera_system.camera().position(),
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
		state.camera_system.camera().position(),
		REACH * 2.0,
		true,
	);

	let state_b = ptr::get_state();
	ptr::get_gamestate()
		.world_mut()
		.make_chunk_meshes(state_b.device(), state_b.queue());
}
/*
/// Performs ray tracing to a cube and determines which of the 27 points (3x3x3 grid) was hit
#[inline]
pub fn raycast_to_cube_point(
	camera: &Camera,
	world: &World,
	max_distance: f32,
) -> Option<(Vec3, (u8, u8, u8))> {
	if !ptr::get_state().is_world_running {
		return None;
	}
	let (block_pos, normal) = raycast_to_block(camera, world, max_distance)?;

	// Get ray details
	let ray_origin = camera.position();
	let ray_dir = camera.forward();

	// Calculate the exact intersection point on the cube's surface
	let t = if normal.x != 0.0 {
		let x = if normal.x > 0.0 {
			block_pos.x + 1.0
		} else {
			block_pos.x
		};
		(x - ray_origin.x) / ray_dir.x
	} else if normal.y != 0.0 {
		let y = if normal.y > 0.0 {
			block_pos.y + 1.0
		} else {
			block_pos.y
		};
		(y - ray_origin.y) / ray_dir.y
	} else {
		let z = if normal.z > 0.0 {
			block_pos.z + 1.0
		} else {
			block_pos.z
		};
		(z - ray_origin.z) / ray_dir.z
	};

	let intersection_point = ray_origin + ray_dir * t;

	// Convert to local block coordinates (0-1 range) then to 3x3x3 grid coordinates
	let local_pos = intersection_point - block_pos;
	let x = (local_pos.x * 3.0).floor().clamp(0.0, 2.0) as u8;
	let y = (local_pos.y * 3.0).floor().clamp(0.0, 2.0) as u8;
	let z = (local_pos.z * 3.0).floor().clamp(0.0, 2.0) as u8;

	Some((block_pos, (x, y, z)))
}

/// Toggles a point in the marching cube that the player is looking at
#[inline]
pub fn toggle_looked_point() {
	let state = ptr::get_state();
	if !state.is_world_running {
		return;
	}
	let camera = &state.camera_system.camera();
	let gamestate = ptr::get_gamestate();
	let world = gamestate.world_mut();

	// Find targeted block and point
	let Some((block_pos, (x, y, z))) = raycast_to_cube_point(camera, world, REACH) else { return; };
	let Some(block) = world.get_block_mut(block_pos) else { return; };
	if block.is_empty() { return; }
	// Convert to marching cube block if needed
	if !block.is_marching() {
		if let Some(march_block) = block.get_march() {
			*block = march_block;
		} else { return; }
	} 
	// Get current point state
	let is_dot = block.get_point(x, y, z).unwrap_or(false); 
	// Toggle
	block.set_point(x, y, z, !is_dot); 
	// Rebuild chunk mesh
	update_chunk_mesh(world, block_pos);
}
*/

