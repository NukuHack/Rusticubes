
use crate::block::math::{BlockPosition, ChunkCoord};
use crate::block::main::{Block, Chunk};
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use ahash::AHasher;
use glam::{IVec3, Vec3};

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Represents the game world containing chunks
#[derive(Debug)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
    pub chunk_generation_queue: Arc<Mutex<Vec<ChunkCoord>>>,
    generated_chunks_receiver: mpsc::Receiver<(ChunkCoord, Chunk)>,
    chunk_generation_sender: mpsc::Sender<(ChunkCoord, Chunk)>,
}

impl World {
    pub fn empty() -> Self {
        let (sender, receiver) = mpsc::channel();
        
        Self {
            chunks: FastMap::with_capacity_and_hasher(10_000, BuildHasherDefault::<AHasher>::default()),
            loaded_chunks: HashSet::with_capacity(10_000),
            chunk_generation_queue: Arc::new(Mutex::new(Vec::new())),
            generated_chunks_receiver: receiver,
            chunk_generation_sender: sender,
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
	pub fn get_block(&self, world_pos: IVec3) -> Block {
		let chunk_coord = ChunkCoord::from_world_pos(world_pos);
		let local_pos: BlockPosition = world_pos.into();
		let index:usize = local_pos.into();

		self.chunks
			.get(&chunk_coord)
			.map(|chunk| chunk.get_block(index))
			.unwrap_or(Block::default())
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
		if chunk.get_block(index) != block {
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

    pub fn start_generation_thread(&self, seed: u32) {
        let queue = self.chunk_generation_queue.clone();
        let sender = self.chunk_generation_sender.clone();
        
        thread::spawn(move || {
            loop {
                let coord = {
                    let mut queue = queue.lock().unwrap();
                    queue.pop()
                };
                
                if let Some(coord) = coord {
                    let chunk = match Chunk::generate(coord, seed) {
                        Some(c) => c,
                        _ => Chunk::empty(),
                    };
                    
                    // Send the generated chunk back to the main thread
                    if sender.send((coord, chunk)).is_err() {
                        // Channel was disconnected, exit thread
                        break;
                    }
                } else {
                    // No work to do, sleep a bit to avoid busy waiting
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });
    }

	#[inline]
	pub fn generate_chunk(&mut self, chunk_coord: ChunkCoord) {
		// Instead of generating immediately, add to queue
		self.loaded_chunks.insert(chunk_coord);
		
		// Insert an empty chunk as placeholder
		self.chunks.insert(chunk_coord, Chunk::empty());
		
		// Add to generation queue
		self.chunk_generation_queue.lock().unwrap().push(chunk_coord);
	}

    pub fn process_generated_chunks(&mut self) {
        // Process all available generated chunks
        while let Ok((chunk_coord, mut chunk)) = self.generated_chunks_receiver.try_recv() {
            let Some(m_chunk) = self.get_chunk_mut(chunk_coord) else { continue; };
            // Move the bind_group from the old chunk to the new one
            chunk.bind_group = std::mem::take(&mut m_chunk.bind_group);

            for coord in chunk_coord.get_adjacent().iter() {
                if let Some(neighbor_chunk) = self.get_chunk_mut(*coord) {
                    neighbor_chunk.final_mesh = false;
                }
            }
            
            self.chunks.insert(chunk_coord, chunk);
        }
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

		// Process any generated chunks that are ready
		self.process_generated_chunks();

		// Load new chunks in range
		for dx in -radius_i32..=radius_i32 {
			for dy in -radius_i32..=radius_i32 {
				for dz in -radius_i32..=radius_i32 {
					if dx * dx + dy * dy + dz * dz > radius_sq { continue; }

					let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
					if !force && self.loaded_chunks.contains(&coord) { continue; }

					self.generate_chunk(coord);
					self.create_bind_group(coord);
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


