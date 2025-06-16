#[allow(unused_imports)]
use super::cube;
use super::cube_math::ChunkCoord;
use crate::camera::Camera;
use crate::cube::{Block, World};
use glam::Vec3;

const REACH: f32 = 6.0;

/// Helper function to update a chunk mesh after modification
fn update_chunk_mesh(world: &mut World, pos: Vec3) {
    let chunk_pos = ChunkCoord::from_world_pos(pos);
    if let Some(chunk) = world.get_chunk_mut(chunk_pos) {
        chunk.make_mesh(
            super::config::get_state().device(),
            super::config::get_state().queue(),
            true,
        );
    }
}

/// Improved raycasting function that finds the first non-empty block and its face
pub fn raycast_to_block(camera: &Camera, world: &World, max_distance: f32) -> Option<(Vec3, Vec3)> {
    let ray_origin = camera.position;
    let ray_dir = camera.forward();

    // Initialize variables for DDA algorithm
    let step = Vec3::new(ray_dir.x.signum(), ray_dir.y.signum(), ray_dir.z.signum());

    let mut block_pos = ray_origin.floor();
    let t_delta = Vec3::new(
        1.0 / ray_dir.x.abs().max(f32::MIN_POSITIVE),
        1.0 / ray_dir.y.abs().max(f32::MIN_POSITIVE),
        1.0 / ray_dir.z.abs().max(f32::MIN_POSITIVE),
    );

    let mut t_max = Vec3::new(
        if step.x > 0.0 {
            block_pos.x + 1.0 - ray_origin.x
        } else {
            ray_origin.x - block_pos.x
        } / ray_dir.x.abs().max(f32::MIN_POSITIVE),
        if step.y > 0.0 {
            block_pos.y + 1.0 - ray_origin.y
        } else {
            ray_origin.y - block_pos.y
        } / ray_dir.y.abs().max(f32::MIN_POSITIVE),
        if step.z > 0.0 {
            block_pos.z + 1.0 - ray_origin.z
        } else {
            ray_origin.z - block_pos.z
        } / ray_dir.z.abs().max(f32::MIN_POSITIVE),
    );

    let mut normal = Vec3::ZERO;
    let mut traveled = 0.0f32;

    while traveled < max_distance {
        // Check current block
        if !world.get_block(block_pos).is_empty() {
            return Some((block_pos, normal));
        }

        // Move to next block boundary
        if t_max.x < t_max.y && t_max.x < t_max.z {
            normal = Vec3::new(-step.x, 0.0, 0.0);
            block_pos.x += step.x;
            traveled = t_max.x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            normal = Vec3::new(0.0, -step.y, 0.0);
            block_pos.y += step.y;
            traveled = t_max.y;
            t_max.y += t_delta.y;
        } else {
            normal = Vec3::new(0.0, 0.0, -step.z);
            block_pos.z += step.z;
            traveled = t_max.z;
            t_max.z += t_delta.z;
        }
    }

    None
}

/// Places a cube on the face of the block the player is looking at
pub fn place_looked_cube() {
    let state = super::config::get_state();
    let camera = &state.camera_system.camera;
    let world = &mut state.data_system.world;

    if let Some((block_pos, normal)) = raycast_to_block(camera, world, REACH) {
        let placement_pos = block_pos + normal;
        world.set_block(placement_pos, Block::new());
        update_chunk_mesh(world, placement_pos);
    }
}

/// Removes the block the player is looking at
pub fn remove_targeted_block() {
    let state = super::config::get_state();
    let camera = &state.camera_system.camera;
    let world = &mut state.data_system.world;

    if let Some((block_pos, _)) = raycast_to_block(camera, world, REACH) {
        world.set_block(block_pos, Block::None);
        update_chunk_mesh(world, block_pos);
    }
}

/// Loads a chunk at the camera's position if not already loaded
#[allow(dead_code)]
pub fn add_def_chunk() {
    let state = super::config::get_state();
    let chunk_pos = ChunkCoord::from_world_pos(state.camera_system.camera.position);

    if state.data_system.world.loaded_chunks.contains(&chunk_pos) {
        return;
    }

    if state.data_system.world.load_chunk(chunk_pos, false) {
        if let Some(chunk) = state.data_system.world.get_chunk_mut(chunk_pos) {
            let state_b = super::config::get_state();
            chunk.make_mesh(state_b.device(), state_b.queue(), true);
        }
    }
}
/// Loads a chunk at the camera's position if not already loaded
pub fn add_full_chunk() {
    let state = super::config::get_state();
    let chunk_pos = ChunkCoord::from_world_pos(state.camera_system.camera.position);

    if state.data_system.world.load_chunk(chunk_pos, true) {
        if let Some(chunk) = state.data_system.world.get_chunk_mut(chunk_pos) {
            let state_b = super::config::get_state();
            chunk.make_mesh(state_b.device(), state_b.queue(), true);
        }
    }
}

/// Loads chunks around the camera in a radius
pub fn update_full_world() {
    let state = super::config::get_state();
    state.data_system.world.update_loaded_chunks(
        state.camera_system.camera.position,
        REACH * 2.0,
        false,
    );

    let state_b = super::config::get_state();
    state
        .data_system
        .world
        .make_chunk_meshes(state_b.device(), state_b.queue());
}

/// Fill chunks around the camera in a radius
pub fn add_full_world() {
    let state = super::config::get_state();
    state.data_system.world.update_loaded_chunks(
        state.camera_system.camera.position,
        REACH * 2.0,
        true,
    );

    let state_b = super::config::get_state();
    state
        .data_system
        .world
        .make_chunk_meshes(state_b.device(), state_b.queue());
}
/// Performs ray tracing to a cube and determines which of the 27 points (3x3x3 grid) was hit
pub fn raycast_to_cube_point(
    camera: &Camera,
    world: &World,
    max_distance: f32,
) -> Option<(Vec3, (u8, u8, u8))> {
    let (block_pos, normal) = raycast_to_block(camera, world, max_distance)?;

    // Get ray details
    let ray_origin = camera.position;
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
pub fn toggle_looked_point() -> Option<(bool, (u8, u8, u8))> {
    let state = super::config::get_state();
    let camera = &state.camera_system.camera;
    let world = &mut state.data_system.world;

    let (block_pos, (x, y, z)) = raycast_to_cube_point(camera, world, REACH)?;

    let block = world.get_block(block_pos);
    if block.is_empty() {
        return None;
    }

    let mut new_block = *block;

    if !new_block.is_marching() {
        new_block = new_block.get_march()?;
    }

    let is_dot = new_block.get_point(x, y, z).unwrap_or(false);
    new_block.set_point(x, y, z, !is_dot);

    world.set_block(block_pos, new_block);

    update_chunk_mesh(world, block_pos);

    Some((is_dot, (x, y, z)))
}
