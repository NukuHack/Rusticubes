use crate::block::math::{BlockPosition, ChunkCoord};
use crate::block::main::{Block, Chunk};
use std::{
	collections::{BinaryHeap, HashMap, HashSet},
	cmp::Ordering as CmpOrdering,
	hash::BuildHasherDefault,
	sync::{
		atomic::{AtomicBool, Ordering},
		mpsc, Arc, Mutex,
	},
	thread,
};
use ahash::AHasher;
use glam::{IVec3, Vec3};

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// A chunk with priority information for loading order
#[derive(Debug, Clone, Copy)]
struct PriorityChunk {
	coord: ChunkCoord,
	distance_sq: i32,
}

impl PriorityChunk {
	/// Creates a new PriorityChunk with calculated distance from center
	fn new(coord: ChunkCoord, center: ChunkCoord) -> Self {
		let (x, y, z) = coord.unpack();
		let (cx, cy, cz) = center.unpack();
		let dx = x - cx;
		let dy = y - cy;
		let dz = z - cz;
		
		Self {
			coord,
			distance_sq: dx * dx + dy * dy + dz * dz,
		}
	}
}

impl PartialEq for PriorityChunk {
	fn eq(&self, other: &Self) -> bool {
		self.distance_sq == other.distance_sq
	}
}

impl Eq for PriorityChunk {}

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
	
	// Chunk generation system
	chunk_generation_queue: Arc<Mutex<BinaryHeap<PriorityChunk>>>,
	generated_chunks_receiver: mpsc::Receiver<(ChunkCoord, Chunk)>,
	chunk_generation_sender: mpsc::Sender<(ChunkCoord, Chunk)>,
	generation_threads_running: Arc<AtomicBool>,
	
	// Configuration
	thread_count: u8,
	seed: u32,
}

impl World {
	/// Creates an empty world
	pub fn empty() -> Self {
		let (sender, receiver) = mpsc::channel();
		
		Self {
			chunks: FastMap::with_capacity_and_hasher(10_000, BuildHasherDefault::<AHasher>::default()),
			loaded_chunks: HashSet::with_capacity(10_000),
			chunk_generation_queue: Arc::new(Mutex::new(BinaryHeap::new())),
			generated_chunks_receiver: receiver,
			chunk_generation_sender: sender,
			generation_threads_running: Arc::new(AtomicBool::new(false)),
			thread_count: 1,
			seed: 0,
		}
	}
	#[inline] pub fn seed(&self) -> u32 { self.seed }
	#[inline] pub fn thread_count(&self) -> u8 { self.thread_count }
	#[inline] pub fn set_seed(&mut self, seed:u32) { self.seed = seed }
	#[inline] pub fn set_thread_count(&mut self, thread_count:u8) { self.thread_count = thread_count }

	#[inline] pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
		self.chunks.get(&coord)
	}

	#[inline] pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
		self.chunks.get_mut(&coord)
	}

	#[inline] pub fn get_block(&self, world_pos: IVec3) -> Block {
		let chunk_coord = ChunkCoord::from_world_pos(world_pos);
		let local_pos: BlockPosition = world_pos.into();
		let index: usize = local_pos.into();

		self.chunks
			.get(&chunk_coord)
			.map(|chunk| chunk.get_block(index))
			.unwrap_or(Block::default())
	}

	#[inline] pub fn set_block(&mut self, world_pos: IVec3, block: Block) {
		let chunk_coord = ChunkCoord::from_world_pos(world_pos);
		let local_pos: BlockPosition = world_pos.into();
		let index: usize = local_pos.into();
		
		// Check if we need to create a new chunk
		if !self.chunks.contains_key(&chunk_coord) {
			self.set_chunk(chunk_coord, Chunk::empty());
		}
		
		let chunk = self.chunks.get_mut(&chunk_coord).expect("Chunk should exist");
		
		// Skip if block is the same
		if chunk.get_block(index) == block {
			return;
		}

		chunk.set_block(index, block);
		
		// If this is a border block, mark adjacent chunks as needing mesh update
		if chunk.is_border_block(local_pos.into()) {
			self.set_adjacent_un_final(chunk_coord);
		}
	}

	/// Starts chunk generation threads
	pub fn start_generation_threads(&mut self, thread_count: u8) {
		if self.generation_threads_running.load(Ordering::Relaxed) {
			return; // Already running
		}

		self.generation_threads_running.store(true, Ordering::Relaxed);
		self.thread_count = thread_count;
		
		for _ in 0..thread_count {
			let queue = Arc::clone(&self.chunk_generation_queue);
			let sender = self.chunk_generation_sender.clone();
			let running = Arc::clone(&self.generation_threads_running);
			let seed = self.seed;
			
			thread::spawn(move || {
				while running.load(Ordering::Relaxed) {
					let priority_chunk = {
						let mut queue = queue.lock().unwrap();
						queue.pop()
					};
					
					if let Some(priority_chunk) = priority_chunk {
						let chunk = Chunk::generate(priority_chunk.coord, seed);
						
						// Send the generated chunk back to the main thread
						if sender.send((priority_chunk.coord, chunk)).is_err() {
							break; // Channel disconnected
						}
					} else {
						// No work, sleep to avoid busy waiting
						thread::sleep(std::time::Duration::from_millis(5));
					}
				}
			});
		}
	}

	#[inline] pub fn stop_generation_threads(&self) {
		self.generation_threads_running.store(false, Ordering::Relaxed);
	}
	
	/// Queues a chunk for generation
	#[inline] fn generate_chunk(&mut self, chunk: PriorityChunk) {
		if !self.loaded_chunks.insert(chunk.coord) { return; } // Skip if already loaded
		
		let mut queue = self.chunk_generation_queue.lock().unwrap();
		// Avoid duplicates in the queue
		if !queue.iter().any(|c| c.coord == chunk.coord) {
			queue.push(chunk);
		}
	}

	/// Updates which chunks are loaded based on player position
	pub fn update_loaded_chunks(&mut self, center: Vec3, radius: f32) {
		let center_coord = ChunkCoord::from_world_posf(center);
		let radius_i32 = radius.round() as i32;
		let radius_sq = (radius * radius) as i32;

		self.unload_distant_chunks(center_coord, radius_sq);
		self.process_generated_chunks();
		self.load_nearby_chunks(center_coord, radius_i32, radius_sq);
	}

	/// Unloads chunks beyond the given radius
	#[inline] fn unload_distant_chunks(&mut self, center: ChunkCoord, radius_sq: i32) {
		let (center_x, center_y, center_z) = center.unpack();
		
		self.loaded_chunks.retain(|&coord| {
			let (x, y, z) = coord.unpack();
			let dx = x - center_x;
			let dy = y - center_y;
			let dz = z - center_z;
			let keep = dx * dx + dy * dy + dz * dz <= radius_sq;
			
			if !keep {
				self.chunks.remove(&coord);
			}
			
			keep
		});
	}

	/// Loads chunks within the given radius
	#[inline] fn load_nearby_chunks(&mut self, center: ChunkCoord, radius: i32, radius_sq: i32) {
		let (center_x, center_y, center_z) = center.unpack();
		let mut chunks_to_load = BinaryHeap::new();
		
		// Generate coordinates for all chunks in the sphere
		for dx in -radius..=radius {
			for dy in -radius..=radius {
				for dz in -radius..=radius {
					let distance_sq = dx * dx + dy * dy + dz * dz;
					if distance_sq > radius_sq {
						continue;
					}

					let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
					if self.loaded_chunks.contains(&coord) { continue; }

					chunks_to_load.push(PriorityChunk::new(coord, center));
				}
			}
		}
		
		// Load chunks in priority order (closest first)
		while let Some(priority_chunk) = chunks_to_load.pop() {
			self.generate_chunk(priority_chunk);
		}
	}

	/// Processes any chunks generated by worker threads
	#[inline] fn process_generated_chunks(&mut self) {
		while let Ok((coord, chunk)) = self.generated_chunks_receiver.try_recv() {
			if !self.loaded_chunks.contains(&coord) { continue; }

			self.set_adjacent_un_final(coord);
			self.chunks.insert(coord, chunk);
			self.create_bind_group(coord);
		}
	}

	/// Marks adjacent chunks as needing mesh updates
	#[inline] pub fn set_adjacent_un_final(&mut self, chunk_coord: ChunkCoord) {
		for coord in chunk_coord.get_adjacent() {
			if let Some(neighbor_chunk) = self.get_chunk_mut(coord) {
				neighbor_chunk.final_mesh = false;
			}
		}
	}

	/// Adds a chunk to the world
	#[inline] pub fn set_chunk(&mut self, chunk_coord: ChunkCoord, chunk: Chunk) {
		self.chunks.insert(chunk_coord, chunk);
		self.loaded_chunks.insert(chunk_coord);
	}
}
