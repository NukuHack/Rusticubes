
use crate::{
	block::{
		math::{BlockPosition, ChunkCoord},
		main::{Block, Chunk},
	},
	world::threading::PriorityChunk,
	item::inventory::ItemContainer,
};
use std::{
	collections::{BinaryHeap, HashMap, HashSet},
	hash::BuildHasherDefault,
	sync::{atomic::{AtomicBool, AtomicUsize}, Arc, Mutex},
};
use ahash::AHasher;
use glam::{IVec3, Vec3};
use crossbeam::channel::{bounded, Sender, Receiver};

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Represents the game world containing chunks
#[derive(Debug)]
pub struct World {
	pub chunks: FastMap<ChunkCoord, Chunk>,
	pub storage_blocks: FastMap<IVec3, Vec<ItemContainer>>,
	pub loaded_chunks: HashSet<ChunkCoord>,
	
	// Chunk generation system
	pub chunk_generation_queue: Arc<Mutex<BinaryHeap<PriorityChunk>>>,
	pub generated_chunks_receiver: Receiver<(ChunkCoord, Chunk)>,
	pub chunk_generation_sender: Sender<(ChunkCoord, Chunk)>,
	pub generation_threads_running: Arc<AtomicBool>,
	pub active_workers: Arc<AtomicUsize>,
	
	// Configuration
	thread_count: u8,
	seed: u32,
}

impl World {
	/// Creates an empty world
	pub fn empty() -> Self {
		let (sender, receiver) = bounded(100); // Use bounded channel to prevent memory explosion
		
		Self {
			chunks: FastMap::with_capacity_and_hasher(10_000, BuildHasherDefault::<AHasher>::default()),
			storage_blocks: FastMap::with_capacity_and_hasher(100, BuildHasherDefault::<AHasher>::default()),
			loaded_chunks: HashSet::with_capacity(10_000),
			chunk_generation_queue: Arc::new(Mutex::new(BinaryHeap::with_capacity(100))),
			generated_chunks_receiver: receiver,
			chunk_generation_sender: sender,
			generation_threads_running: Arc::new(AtomicBool::new(false)),
			active_workers: Arc::new(AtomicUsize::new(0)),
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

	/// Creates a storage container at the specified position
	#[inline] pub fn create_storage(&mut self, position: IVec3, data: Vec<ItemContainer>) {
		self.storage_blocks.insert(position, data);
	}
	/// Removes a storage container
	#[inline] pub fn remove_storage(&mut self, position: IVec3) -> Option<Vec<ItemContainer>> {
		self.storage_blocks.remove(&position)
	}
	/// Gets a storage container (immutable)
	#[inline] pub fn get_storage(&self, position: IVec3) -> Option<&Vec<ItemContainer>> {
		self.storage_blocks.get(&position)
	}
	/// Gets a storage container (mutable)
	#[inline] pub fn get_storage_mut(&mut self, position: IVec3) -> Option<&mut Vec<ItemContainer>> {
		self.storage_blocks.get_mut(&position)
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
		
		let chunk = self.chunks.get(&chunk_coord).expect("Chunk should exist");
		let old_block = chunk.get_block(index);
		// Skip if block is the same
		if old_block == block { return; }

		if old_block.is_storage() {
			self.storage_blocks.remove(&world_pos);
		}
		
		// If placing a new storage block, initialize its storage
		if block.is_storage() {
			let slot = block.get_storage();
			self.create_storage(world_pos, vec![ItemContainer::new(slot.rows(), slot.cols())]);
		}

		let chunk = self.chunks.get_mut(&chunk_coord).expect("Chunk should exist");
		chunk.set_block(index, block);
		
		// If this is a border block, mark adjacent chunks as needing mesh update
		if chunk.is_border_block(local_pos.into()) {
			self.set_adjacent_un_final(chunk_coord);
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

		// Unload distant storage blocks
		self.storage_blocks.retain(|pos, _| {
			let chunk_coord = ChunkCoord::from_world_pos(*pos);
			let (x, y, z) = chunk_coord.unpack();
			let dx = x - center_x;
			let dy = y - center_y;
			let dz = z - center_z;
			dx * dx + dy * dy + dz * dz <= radius_sq
		});
		
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

	/// Loads chunks within the given radius using a more efficient spiral pattern
	fn load_nearby_chunks(&mut self, center: ChunkCoord, radius: i32, radius_sq: i32) {
		let (center_x, center_y, center_z) = center.unpack();
		
		// Spiral out from center for better prioritization
		let mut chunks_to_load = Vec::with_capacity((radius * 2).pow(3) as usize);
		
		// Spiral pattern implementation
		let mut x = 0;
		let mut z = 0;
		let mut dx = 0;
		let mut dz = -1;
		let max = radius.max(1) * radius.max(1);
		
		for _ in 0..max {
			if (-radius < x && x <= radius) && (-radius < z && z <= radius) {
				for dy in -radius..=radius {
					let distance_sq = x*x + dy*dy + z*z;
					if distance_sq > radius_sq {
						continue;
					}
					
					let coord = ChunkCoord::new(center_x + x, center_y + dy, center_z + z);
					if !self.loaded_chunks.contains(&coord) {
						chunks_to_load.push(PriorityChunk::new(coord, center));
					}
				}
			}
			
			if x == z || (x < 0 && x == -z) || (x > 0 && x == 1-z) {
				let tmp = dx;
				dx = -dz;
				dz = tmp;
			}
			
			x += dx;
			z += dz;
		}
		
		// Sort by distance (closest first)
		chunks_to_load.sort_unstable();
		
		// Queue chunks for generation
		for chunk in chunks_to_load {
			self.generate_chunk(chunk);
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
