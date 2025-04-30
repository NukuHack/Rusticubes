use super::cube::ChunkCoord;
use crate::camera::Camera;
use crate::cube::{Block, World};
use glam::Vec3;

/// Parses a 4-character string into (x,y,z,value) for block point manipulation
/// Format: "XYZV" where X,Y,Z are digits 0-2 and V is 0 or 1
fn parse_block_point_input(raw_data: &str) -> Option<(u8, u8, u8, bool)> {
    if raw_data.len() != 4 {
        eprintln!("Input must be exactly 4 characters (XYZV)");
        return None;
    }

    let parse_digit = |c: char| -> Option<u8> {
        c.to_digit(10)
            .and_then(|d| if d <= 2 { Some(d as u8) } else { None })
    };

    let mut chars = raw_data.chars();
    let x = parse_digit(chars.next()?)?;
    let y = parse_digit(chars.next()?)?;
    let z = parse_digit(chars.next()?)?;
    let val = chars.next()? == '1';

    Some((x, y, z, val))
}

/// Helper function to update a chunk mesh after modification
unsafe fn update_chunk_mesh(world: &mut World, pos: Vec3, is_marching: bool) {
    let chunk_pos = ChunkCoord::from_world_pos(pos);
    if let Some(chunk) = world.get_chunk_mut(chunk_pos) {
        unsafe {
            let state = super::get_state();
            chunk.make_mesh(state.device(), state.queue(), is_marching);
        };
    }
}

/// Updates a marching cubes block at (0,0,0) with new point data
pub fn march_def_cube(raw_data: &str) -> bool {
    let Some((x, y, z, value)) = parse_block_point_input(raw_data) else {
        return false;
    };

    unsafe {
        let state = super::get_state();
        let pos = Vec3::ZERO;
        let default_block = &mut Block::None;

        let block = state
            .data_system
            .world
            .get_block_mut(pos)
            .unwrap_or(default_block);

        if block.is_empty() {
            return false;
        }

        block.set_point(x, y, z, value);
        update_chunk_mesh(&mut state.data_system.world, pos, true);
        true
    }
}

/// Places a simple cube at (0,0,0)
pub fn place_default_cube() {
    unsafe {
        let state = super::get_state();
        let pos = Vec3::ZERO;
        state.data_system.world.set_block(pos, Block::new());
        update_chunk_mesh(&mut state.data_system.world, pos, false);
    }
}

/// Places a complex cube at (0,0,0)
pub fn place_marched_cube() {
    unsafe {
        let state = super::get_state();
        let pos = Vec3::ZERO;
        state.data_system.world.set_block(pos, Block::new_dot());
        update_chunk_mesh(&mut state.data_system.world, pos, false);
    }
}

/// Loads a chunk at the camera's position if not already loaded
pub fn add_def_chunk() {
    unsafe {
        let state = super::get_state();
        let chunk_pos = ChunkCoord::from_world_pos(state.camera_system.camera.position);

        if state.data_system.world.loaded_chunks.contains(&chunk_pos) {
            return;
        }

        if state.data_system.world.load_chunk(chunk_pos) {
            if let Some(chunk) = state.data_system.world.get_chunk_mut(chunk_pos) {
                let state_b = super::get_state();
                chunk.make_mesh(state_b.device(), state_b.queue(), true);
            }
        }
    }
}

/// Loads chunks around the camera in a radius
pub fn add_full_world() {
    unsafe {
        let state = super::get_state();
        state
            .data_system
            .world
            .update_loaded_chunks(state.camera_system.camera.position, 6u32);

        let state_b = super::get_state();
        state
            .data_system
            .world
            .make_chunk_meshes(state_b.device(), state_b.queue());
    }
}

/// Improved raycasting function that finds the first non-empty block and its face
pub fn raycast_to_block(camera: &Camera, world: &World, max_distance: f32) -> Option<(Vec3, Vec3)> {
    // Adjust ray origin to block center
    let ray_origin = camera.position;
    let ray_dir = camera.forward();

    // Initialize variables for DDA algorithm
    let step = Vec3::new(
        ray_dir.x.signum() as i32 as f32,
        ray_dir.y.signum() as i32 as f32,
        ray_dir.z.signum() as i32 as f32,
    );

    let mut block_pos = Vec3::new(
        ray_origin.x.floor(),
        ray_origin.y.floor(),
        ray_origin.z.floor(),
    );

    let t_delta = Vec3::new(
        1.0 / ray_dir.x.abs().max(f32::MIN_POSITIVE),
        1.0 / ray_dir.y.abs().max(f32::MIN_POSITIVE),
        1.0 / ray_dir.z.abs().max(f32::MIN_POSITIVE),
    );

    let mut t_max = Vec3::new(
        if step.x > 0.0 {
            block_pos.x + 1.0 - ray_origin.x
        } else {
            block_pos.x - ray_origin.x
        }
        .abs()
            / ray_dir.x.abs().max(f32::MIN_POSITIVE),
        if step.y > 0.0 {
            block_pos.y + 1.0 - ray_origin.y
        } else {
            block_pos.y - ray_origin.y
        }
        .abs()
            / ray_dir.y.abs().max(f32::MIN_POSITIVE),
        if step.z > 0.0 {
            block_pos.z + 1.0 - ray_origin.z
        } else {
            block_pos.z - ray_origin.z
        }
        .abs()
            / ray_dir.z.abs().max(f32::MIN_POSITIVE),
    );

    let mut traveled = 0.0f32;
    let mut normal = Vec3::ZERO;

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
    unsafe {
        let state = super::get_state();
        let camera = &state.camera_system.camera;
        let world = &state.data_system.world;

        if let Some((block_pos, normal)) = raycast_to_block(camera, world, 6.0) {
            let placement_pos = block_pos + normal;
            state
                .data_system
                .world
                .set_block(placement_pos, Block::new());
            update_chunk_mesh(&mut state.data_system.world, placement_pos, false);
        }
    }
}

/// Removes the block the player is looking at
pub fn remove_targeted_block() {
    unsafe {
        let state = super::get_state();
        let camera = &state.camera_system.camera;
        let world = &state.data_system.world;

        if let Some((block_pos, _)) = raycast_to_block(camera, world, 10.0) {
            state
                .data_system
                .world
                .set_block(block_pos, Block::default());
            update_chunk_mesh(&mut state.data_system.world, block_pos, false);
        }
    }
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
pub fn toggle_looked_point() -> Option<(Option<bool>, (u8, u8, u8))> {
    unsafe {
        let state = super::get_state();
        let camera = &state.camera_system.camera;
        let world = &mut state.data_system.world;

        let (block_pos, (x, y, z)) = raycast_to_cube_point(camera, world, 6.0)?;
        let block = world.get_block_mut(block_pos)?;

        if block.is_empty() {
            return None;
        }

        if !block.is_marching() {
            let marched_block = block.get_march()?;
            world.set_block(block_pos, marched_block);
        }

        let block = world.get_block_mut(block_pos)?;
        let current = block.get_point(x, y, z);
        block.set_point(x, y, z, !current.unwrap_or(false));

        update_chunk_mesh(world, block_pos, true);
        Some((current, (x, y, z)))
    }
}
