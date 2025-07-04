
use crate::config;
use crate::block::main::Block;
use crate::block::main::Chunk;
use crate::block::math::ChunkCoord;
#[allow(unused_imports)]
use crate::debug;
use ahash::AHasher;
use glam::Vec3;
use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};
use wgpu::util::DeviceExt;

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Represents the game world containing chunks
#[derive(Debug, Clone)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
}

#[allow(dead_code)]
impl World {
    /// Creates an empty world
    #[inline]
    pub fn empty() -> Self {
        Self {
            chunks: FastMap::with_capacity_and_hasher(10_000, BuildHasherDefault::<AHasher>::default()),
            loaded_chunks: HashSet::with_capacity(10_000),
        }
    }

    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    #[inline]
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    #[inline]
    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
        self.chunks.get_mut(&coord)
    }

    #[inline]
    pub fn get_block(&self, world_pos: Vec3) -> &Block {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        let local_pos = Chunk::world_to_local_pos(world_pos);
        let index:usize = local_pos.into();

        self.chunks
            .get(&chunk_coord)
            .map(|chunk| chunk.get_block(index))
            .unwrap_or(&Block::None)
    }

    #[inline]
    pub fn get_block_mut(&mut self, world_pos: Vec3) -> Option<&mut Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        let local_pos = Chunk::world_to_local_pos(world_pos);
        let index: usize = local_pos.into();

        self.chunks
            .get_mut(&chunk_coord)
            .map(|chunk| {
                let palette_idx = chunk.storage.get(index);
                &mut chunk.palette[palette_idx as usize]
            })
    }

    pub fn set_block(&mut self, world_pos: Vec3, block: Block) {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);

        if !self.chunks.contains_key(&chunk_coord) {
            self.set_chunk(chunk_coord, Chunk::empty());
        }

        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            let local_pos = Chunk::world_to_local_pos(world_pos);
            let index:usize = local_pos.into();

            // Only set if the block is actually different
            if chunk.get_block(index) != &block {
                chunk.set_block(index, block);
            }
        }
    }

    /// Loads a chunk from storage
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoord, force: bool) -> bool {
        let state = config::get_state();
        let device = &state.render_context.device;
        let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

        let mut chunk = Chunk::empty();
        if force {
            chunk = match Chunk::load() {
                Some(c) => c,
                None => return false,
            };
        }

        // For palette-based chunks, we need a more sophisticated comparison
        if let Some(existing_chunk) = self.get_chunk(chunk_coord) {
            // Compare palette and storage instead of individual blocks
            if existing_chunk.palette == chunk.palette && existing_chunk.storage == chunk.storage {
                return false;
            }
        }

        // Create position buffer
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Position Buffer"),
            contents: bytemuck::cast_slice(&[
                chunk_coord.into(),
                0.0 as u64,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: chunk_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: position_buffer.as_entire_binding(),
            }],
            label: Some("chunk_bind_group"),
        });

        self.chunks.insert(
            chunk_coord,
            Chunk {
                bind_group: Some(bind_group),
                ..chunk
            },
        );
        self.loaded_chunks.insert(chunk_coord);

        true
    }

    /// Updates loaded chunks based on player position
    pub fn update_loaded_chunks(&mut self, center: Vec3, radius: f32, force: bool) {
        let center_coord = ChunkCoord::from_world_pos(center);
        let (center_x, center_y, center_z) = center_coord.unpack();
        let radius_i32 = radius as i32;
        let radius_sq = (radius * radius) as i32;

        // Unload distant chunks
        let mut to_unload = Vec::new();
        for &coord in &self.loaded_chunks {
            let (x, y, z) = coord.unpack();
            let dx = x - center_x;
            let dy = y - center_y;
            let dz = z - center_z;

            if dx * dx + dy * dy + dz * dz > radius_sq {
                to_unload.push(coord);
            }
        }

        for coord in to_unload {
            self.unload_chunk(coord);
        }

        // Load new chunks in range
        for dx in -radius_i32..=radius_i32 {
            for dy in -radius_i32..=radius_i32 {
                for dz in -radius_i32..=radius_i32 {
                    if dx * dx + dy * dy + dz * dz > radius_sq {
                        continue;
                    }

                    let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
                    if force || !self.loaded_chunks.contains(&coord) {
                        self.load_chunk(coord, false);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn set_chunk(&mut self, chunk_coord: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(chunk_coord, chunk);
        self.loaded_chunks.insert(chunk_coord);
    }

    #[inline]
    pub fn unload_chunk(&mut self, chunk_coord: ChunkCoord) {
        self.chunks.remove(&chunk_coord);
        self.loaded_chunks.remove(&chunk_coord);
    }

    /// Generates meshes for all dirty chunks
    #[inline]
    pub fn make_chunk_meshes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
//        let timer = std::time::Instant::now();
        //let mut chunk_times = debug::RunningAverage::default();

        for chunk in self.chunks.values_mut() {
            // Skip empty chunks entirely
            if chunk.is_empty() {
                continue;
            }

            //let chunk_timer = std::time::Instant::now();
            chunk.make_mesh(device, queue, false);
            //let elapsed_micros = chunk_timer.elapsed().as_micros() as f32;
            //chunk_times.add(elapsed_micros.into());
        }
/*
        println!(
            "World mesh_gen_time: {:.2}ms",
            timer.elapsed().as_secs_f32() * 1000.0
        );
*/
    }
}


