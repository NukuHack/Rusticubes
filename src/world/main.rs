
use crate::ext::ptr;
use crate::block::math::{BlockPosition, ChunkCoord};
use crate::block::main::{Block, Chunk};
use ahash::AHasher;
use glam::{IVec3, Vec3};
use std::{
	collections::{HashMap, HashSet},
	hash::BuildHasherDefault,
};

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

	#[inline] pub fn chunk_count(&self) -> usize {
		self.chunks.len()
	}

	#[inline] pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
		self.chunks.get(&coord)
	}

	#[inline] pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
		self.chunks.get_mut(&coord)
	}

	#[inline]
	pub fn get_block(&self, world_pos: IVec3) -> &Block {
		let chunk_coord = ChunkCoord::from_world_pos(world_pos);
		let local_pos: BlockPosition = world_pos.into();
		let index:usize = local_pos.into();

		self.chunks
			.get(&chunk_coord)
			.map(|chunk| chunk.get_block(index))
			.unwrap_or(&Block::None)
	}

	#[inline]
	pub fn get_block_mut(&mut self, world_pos: IVec3) -> Option<&mut Block> {
		let chunk_coord = ChunkCoord::from_world_pos(world_pos);
		let local_pos: BlockPosition = world_pos.into();
		let index: usize = local_pos.into();

		self.chunks
			.get_mut(&chunk_coord)
			.map(|chunk| {
				let palette_idx = chunk.storage.get(index);
				&mut chunk.palette[palette_idx as usize]
			})
	}
	#[inline]
	pub fn set_block(&mut self, world_pos: IVec3, block: Block) {
		let chunk_coord = ChunkCoord::from_world_pos(world_pos);
		
		// Get immutable access first to check conditions
		let needs_new_chunk = !self.chunks.contains_key(&chunk_coord);
		let is_border_block = self.get_chunk(chunk_coord)
			.map(|chunk| chunk.is_border_block(world_pos.into()))
			.unwrap_or(false);
		
		// Only get mutable access if we actually need to modify
		let chunk = if needs_new_chunk {
			self.set_chunk(chunk_coord, Chunk::empty());
			self.get_chunk_mut(chunk_coord).expect("Chunk should exist after insertion")
		} else {
			self.get_chunk_mut(chunk_coord).expect("Chunk should exist")
		};
		
		let local_pos: BlockPosition = world_pos.into();
		let index: usize = local_pos.into();
		
		// Only proceed if the block is actually different
		if chunk.get_block(index) != &block {
			chunk.set_block(index, block);
			if is_border_block {
				for coord in chunk_coord.get_adjacent().iter() {
				    if let Some(neighbor_chunk) = self.get_chunk_mut(*coord) {
						neighbor_chunk.final_mesh = false;
				    }
				}
			}
		}
	}
	#[inline]
	/// Loads a new chunk
	pub fn load_chunk(&mut self, chunk_coord: ChunkCoord) {
		let chunk = Chunk::new(1u16);

		self.loaded_chunks.insert(chunk_coord);
		self.chunks.insert(
			chunk_coord,
			chunk
		);
	}
	#[inline]
	/// Loads a chunk from storage
	pub fn generate_chunk(&mut self, chunk_coord: ChunkCoord, seed: u32) {
		let chunk = match Chunk::generate(chunk_coord, seed) {
			Some(c) => c,
			_ => Chunk::empty(),
		};

		self.loaded_chunks.insert(chunk_coord);
		self.chunks.insert(
			chunk_coord,
			chunk
		);
	}


	/// Updates loaded chunks based on player position
	pub fn update_loaded_chunks(&mut self, center: Vec3, radius: f32, force: bool) {
		let center_coord = ChunkCoord::from_world_posf(center);
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
		let seed = *ptr::get_gamestate().seed();
		// Load new chunks in range
		for dx in -radius_i32..=radius_i32 {
			for dy in -radius_i32..=radius_i32 {
				for dz in -radius_i32..=radius_i32 {
					if dx * dx + dy * dy + dz * dz > radius_sq {
						continue;
					}

					let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
					if force || !self.loaded_chunks.contains(&coord) {
						self.generate_chunk(coord, seed);
						self.create_bind_group(coord);
					}
				}
			}
		}
	}

	#[inline] pub fn set_chunk(&mut self, chunk_coord: ChunkCoord, chunk: Chunk) {
		self.chunks.insert(chunk_coord, chunk);
		self.loaded_chunks.insert(chunk_coord);
	}

	#[inline] pub fn unload_chunk(&mut self, chunk_coord: ChunkCoord) {
		self.chunks.remove(&chunk_coord);
		self.loaded_chunks.remove(&chunk_coord);
	}
}


