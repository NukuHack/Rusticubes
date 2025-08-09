
use crate::block::math::{BlockPosition, ChunkCoord};
use crate::block::main::{Block, Chunk};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::hash::BuildHasherDefault;
use std::sync::{mpsc, Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::cmp::Ordering as CmpOrdering;
use ahash::AHasher;
use glam::{IVec3, Vec3};

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

#[derive(Debug, Clone, Copy)]
struct PriorityChunk {
	coord: ChunkCoord,
	distance_sq: i32,
}
impl PartialEq for PriorityChunk {
	fn eq(&self, other: &Self) -> bool {
		self.distance_sq == other.distance_sq
	}
}
impl Eq for PriorityChunk {

}
impl PartialOrd for PriorityChunk {
	fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
		Some(self.cmp(other))
	}
}
impl Ord for PriorityChunk {
	fn cmp(&self, other: &Self) -> CmpOrdering {
		// Reverse ordering for min-heap (closest chunks first)
		other.distance_sq.cmp(&self.distance_sq)
	}
}

/// Represents the game world containing chunks
#[derive(Debug)]
pub struct World {
	pub chunks: FastMap<ChunkCoord, Chunk>,
	pub loaded_chunks: HashSet<ChunkCoord>,
	chunk_generation_queue: Arc<Mutex<BinaryHeap<PriorityChunk>>>,
	generated_chunks_receiver: mpsc::Receiver<(ChunkCoord, Chunk)>,
	chunk_generation_sender: mpsc::Sender<(ChunkCoord, Chunk)>,
	generation_threads_running: Arc<AtomicBool>,
}

impl World {
	pub fn empty() -> Self {
		let (sender, receiver) = mpsc::channel();
		
		Self {
			chunks: FastMap::with_capacity_and_hasher(10_000, BuildHasherDefault::<AHasher>::default()),
			loaded_chunks: HashSet::with_capacity(10_000),
			chunk_generation_queue: Arc::new(Mutex::new(BinaryHeap::new())),
			generated_chunks_receiver: receiver,
			chunk_generation_sender: sender,
			generation_threads_running: Arc::new(AtomicBool::new(false)),
		}
	}

	pub fn start_generation_threads(&self, seed: u32, thread_count: usize) {
		if self.generation_threads_running.load(Ordering::Relaxed) { return; } // Already running

		self.generation_threads_running.store(true, Ordering::Relaxed);
		
		for _ in 0..thread_count {
			let queue = self.chunk_generation_queue.clone();
			let sender = self.chunk_generation_sender.clone();
			let running = self.generation_threads_running.clone();
			
			thread::spawn(move || {
				while running.load(Ordering::Relaxed) {
					let priority_chunk = {
						let mut queue = queue.lock().unwrap();
						queue.pop()
					};
					
					if let Some(priority_chunk) = priority_chunk {
						let chunk = match Chunk::generate(priority_chunk.coord, seed) {
							Some(c) => c,
							_ => Chunk::empty(),
						};
						
						// Send the generated chunk back to the main thread
						if sender.send((priority_chunk.coord, chunk)).is_err() {
							// Channel was disconnected, exit thread
							break;
						}
					} else {
						// No work to do, sleep a bit to avoid busy waiting
						thread::sleep(std::time::Duration::from_millis(5));
					}
				}
			});
		}
	}

	pub fn stop_generation_threads(&self) {
		self.generation_threads_running.store(false, Ordering::Relaxed);
	}
	
	#[inline]
	pub fn generate_chunk_with_priority(&mut self, chunk_coord: ChunkCoord, center: Vec3) {
		if self.loaded_chunks.contains(&chunk_coord) {
			return; // Skip if already loaded
		}

		let (cx, cy, cz) = chunk_coord.unpack();
		let center_coord = ChunkCoord::from_world_posf(center);
		let (pcx, pcy, pcz) = center_coord.unpack();
		
		let dx = cx - pcx;
		let dy = cy - pcy;
		let dz = cz - pcz;
		let distance_sq = dx * dx + dy * dy + dz * dz;
		
		self.loaded_chunks.insert(chunk_coord);
		self.chunks.insert(chunk_coord, Chunk::empty());
		
		let priority_chunk = PriorityChunk {
			coord: chunk_coord,
			distance_sq,
		};
		
		let mut queue = self.chunk_generation_queue.lock().unwrap();
		// Optional: Check if the chunk is already in the queue to avoid duplicates
		if !queue.iter().any(|c| c.coord == chunk_coord) {
			queue.push(priority_chunk);
		}
	}

	pub fn update_loaded_chunks(&mut self, center: Vec3, radius: f32, force: bool) {
		let center_coord = ChunkCoord::from_world_posf(center);
		let (center_x, center_y, center_z) = center_coord.unpack();
		let radius_i32 = radius.round() as i32; // Better rounding
		let radius_sq = (radius * radius) as i32;

		// Unload distant chunks
		let to_unload: Vec<_> = self.loaded_chunks.iter()
			.filter(|&&coord| {
				let (x, y, z) = coord.unpack();
				let dx = x - center_x;
				let dy = y - center_y;
				let dz = z - center_z;
				dx * dx + dy * dy + dz * dz > radius_sq
			})
			.cloned()
			.collect();

		for coord in to_unload {
			self.unload_chunk(coord);
		}

		self.process_generated_chunks();

		// Use BinaryHeap for priority (note: BinaryHeap is a max-heap, so reverse ordering)
		let mut chunks_to_load = std::collections::BinaryHeap::new();
		
		for dx in -radius_i32..=radius_i32 {
			for dy in -radius_i32..=radius_i32 {
				for dz in -radius_i32..=radius_i32 {
					let distance_sq = dx * dx + dy * dy + dz * dz;
					if distance_sq > radius_sq { continue; }

					let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
					if !force && self.loaded_chunks.contains(&coord) { continue; }

					chunks_to_load.push(PriorityChunk {
						coord,
						distance_sq,
					});
				}
			}
		}
		
		// Load chunks in priority order (closest first)
		while let Some(priority_chunk) = chunks_to_load.pop() {
			self.generate_chunk_with_priority(priority_chunk.coord, center);
			self.create_bind_group(priority_chunk.coord);
		}
	}

	// Keep existing methods unchanged
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
		if chunk.get_block(index) == block { return; }

		chunk.set_block(index, block);
		if !is_border_block { return; }

		self.set_adjacent_un_final(chunk_coord);
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

	pub fn process_generated_chunks(&mut self) {
		// Process all available generated chunks
		while let Ok((chunk_coord, mut chunk)) = self.generated_chunks_receiver.try_recv() {
			let Some(m_chunk) = self.get_chunk_mut(chunk_coord) else { continue; };
			// Move the bind_group from the old chunk to the new one
			chunk.bind_group = std::mem::take(&mut m_chunk.bind_group);

			self.set_adjacent_un_final(chunk_coord);
			
			self.chunks.insert(chunk_coord, chunk);
		}
	}

	#[inline] pub fn set_adjacent_un_final(&mut self, chunk_coord: ChunkCoord) {
		for coord in chunk_coord.get_adjacent().iter() {
			let Some(neighbor_chunk) = self.get_chunk_mut(*coord) else { continue; };
			
			neighbor_chunk.final_mesh = false;
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
