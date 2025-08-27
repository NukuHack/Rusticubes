
use crate::{
	item::inventory::Slot,
	item::items::lut_by_name,
	block::extra::get_item_name_from_block_id,
	block::math::{self, ChunkCoord, LocalPos, BlockRotation},
	block::storage::BlockStorage,
	block::entity::EntityStorage,
	utils::rng::{Noise},
	render::meshing::GeometryBuffer,
};
use glam::IVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material(pub u16);
impl Material {
	#[inline] pub const fn inner(&self) -> u16 {
		self.0
	}
	#[inline] pub const fn from(val:u16) -> Self {
		Self(val)
	}
}

/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Block {
	pub material: Material,
	pub rotation: BlockRotation, // material, rotation
}

impl Block {
	/// Creates a default empty block
	#[inline] pub const fn default() -> Self {
		Self::new(Material(1))
	}
	/// Creates a new simple block with default material
	#[inline] pub const fn new(material: Material) -> Self {
		Self { material, rotation: BlockRotation::XPLUS_YPLUS }
	}
	#[inline] pub const fn from(material: Material, rotation:BlockRotation) -> Self {
		Self { material, rotation }
	}
	/// Extracts rotation
	#[inline] pub const fn get_rotation(&self) -> BlockRotation {
		self.rotation
	}
	/// Rotates the block around an axis by N 90° steps
	#[inline]
	pub fn rotate(&mut self, axis: math::AxisBasic, steps: u8) {
		self.set_rotation(self.rotation.rotate(axis, steps));
	}
	#[inline] pub const fn is_empty(&self) -> bool {
		self.material.inner() == 1u16 // this should be reworked as like "is not rendered?"
	}
	#[inline] pub const fn material(&self) -> Material {
		self.material
	}
	#[inline]
	pub fn set_material(&mut self, material: Material) {
		self.material = material;
	}
	/// Sets all rotation axes at once
	#[inline]
	pub fn set_rotation(&mut self, rotation: BlockRotation) {
		self.rotation = rotation;
	}
	/// get the item and check if is storage
	#[inline]
	pub fn is_storage(&self) -> bool {
		let item_name = get_item_name_from_block_id(self.material.inner());
		let item = lut_by_name(&item_name);
		item.is_storage()
	}
	#[inline]
	pub fn get_storage(&self) -> Slot {
		let item_name = get_item_name_from_block_id(self.material.inner());
		let item = lut_by_name(&item_name);
		item.data.clone().expect("Shold check first").get_slot().expect("Shold check the data first")
	}
}

/// Represents a chunk of blocks in the world
#[derive(PartialEq, Debug)]
pub struct Chunk {
	storage: BlockStorage,
	entities: EntityStorage,

	pub dirty: bool,
	pub final_mesh: bool,
	finished_gen: bool,

	mesh: Option<GeometryBuffer>,
	bind_group: Option<wgpu::BindGroup>,
}
impl Clone for Chunk {
	fn clone(&self) -> Self {
		Self {
			storage: self.storage.clone(),
			entities: self.entities.clone(),
			
			dirty: self.dirty,
			final_mesh: self.final_mesh,
			finished_gen: self.finished_gen,
			
			// These are typically not cloned as they're GPU resources
			mesh: None,
			bind_group: None,
		}
	}
}
impl Chunk {
	pub const SIZE: usize = 32;
	pub const SIZE_I: i32 = Self::SIZE as i32;
	pub const SIZE_F: f32 = Self::SIZE as f32;
	pub const VOLUME: usize = Self::SIZE * Self::SIZE * Self::SIZE; // 32K+

	/// Creates an empty chunk (all blocks are air)
	#[inline] pub fn empty() -> Self {
		Self {
			storage: BlockStorage::empty(),
			entities: EntityStorage::Empty,

			dirty: false,
			final_mesh: false,
			finished_gen: false,

			mesh: None,
			bind_group: None,
		}
	}

	/// Creates a new filled chunk (all blocks initialized to `Block::new(<mat>)`)
	#[inline] pub fn new(mat: u16) -> Self {
		let block = Block::new(Material(mat));
		Self {
			storage: BlockStorage::uniform(block),
			entities: EntityStorage::Empty,

			dirty: true,
			final_mesh: false,
			finished_gen: true,

			mesh: None,
			bind_group: None,
		}
	}

	#[inline] pub fn from_storage(storage: BlockStorage) -> Self {
		Self {
			storage,
			entities: EntityStorage::Empty,

			dirty: true,
			final_mesh: false,
			finished_gen: true,

			mesh: None,
			bind_group: None,
		}
	}
	#[inline] pub fn from_storage_and_entities(storage: BlockStorage, entities: EntityStorage) -> Self {
		Self {
			storage,
			entities,

			dirty: true,
			final_mesh: false,
			finished_gen: true,

			mesh: None,
			bind_group: None,
		}
	}

	pub fn generate(coord: ChunkCoord, seed: u32) -> Self {
		if coord.y() > 6i32 { return Self::empty(); }
		if coord.y() <= -2i32 { return Self::new(2u16); }
		
		let noise_gen = Noise::new(seed);
		let (world_x, world_y, world_z) = coord.unpack_to_worldpos();
		let mut chunk = Self::empty();
		let block = Block::new(Material(2u16));
		
		// Pre-calculate all noise values for this chunk's XZ plane
		for x in 0..Self::SIZE {
			for z in 0..Self::SIZE {
				let pos_x: i32 = world_x + x as i32;
				let pos_z: i32 = world_z + z as i32;
				
				// Get noise value and scale it to a reasonable height range
				let noise: f32 = noise_gen.terrain_noise_2d(pos_x, pos_z);
				let final_noise = noise * (8 * Chunk::SIZE) as f32;
				
				for y in 0..Self::SIZE {
					let pos_y = world_y + y as i32;
					// If this block is under or in terrain height, make it solid
					if pos_y <= final_noise as i32 {
						// Correct block indexing : BlockPosition
						let idx: LocalPos = LocalPos::from((x, y, z));
						chunk.set_block(usize::from(idx), block); // Set to solid
					}
					// Else leave as air
				}
			}
		}
		chunk.finished_gen = true;
		chunk
	}

	#[inline] pub fn get_block(&self, index: usize) -> Block {
		self.storage.get(index)
	}

	/// Checks if the chunk is completely empty (all blocks are air)
	#[inline]
	pub fn is_empty(&self) -> bool {
		match &self.storage {
			BlockStorage::Uniform { block } => block.is_empty(),
			_ => {
				// For compact/sparse storage, check if all blocks are air
				for i in 0..Self::VOLUME {
					if !self.storage.get(i).is_empty() {
						return false;
					}
				}
				true
			}
		}
	}

	/// Checks if the chunk is completely full (all blocks are not air)
	#[inline]
	pub fn is_full(&self) -> bool {
		match &self.storage {
			BlockStorage::Uniform { block } => !block.is_empty(),
			_ => {
				// For compact/sparse storage, check if all blocks are non-air
				for i in 0..Self::VOLUME {
					if self.storage.get(i).is_empty() {
						return false;
					}
				}
				true
			}
		}
	}

	/// Sets a block at the given index
	pub fn set_block(&mut self, index: usize, block: Block) {
		let old_block = self.get_block(index);
		if old_block.is_storage() {
			self.remove_entity(index.into());
		}

		self.storage.set(index, block);
		self.dirty = true;

		// Periodically optimize storage to avoid bloat
		// Only optimize sparse storage periodically to avoid performance hits
		static mut OPTIMIZATION_COUNTER: usize = 0;
		unsafe {
			OPTIMIZATION_COUNTER += 1;
			if OPTIMIZATION_COUNTER % 100 == 0 {
				self.storage.optimize();
			}
		}
	}

	/// Checks if a block position is empty or outside the chunk
	#[inline]
	pub fn is_block_cull(&self, pos: IVec3) -> bool {
		let idx: usize = usize::from(LocalPos::from(pos));
		self.get_block(idx).is_empty()
	}

	#[inline]
	pub const fn contains_position(&self, pos: IVec3) -> bool {
		// Check if position is outside chunk bounds
		if pos.x < 0
			|| pos.y < 0
			|| pos.z < 0
			|| pos.x >= Self::SIZE_I
			|| pos.y >= Self::SIZE_I
			|| pos.z >= Self::SIZE_I
		{
			return false;
		}
		true
	}

	#[inline]
	pub const fn is_border_block(&self, pos: IVec3) -> bool {
		// Check if position barely inside the chunk
		if pos.x == 0 || pos.x == Self::SIZE_I-1
			|| pos.y == 0 || pos.y == Self::SIZE_I-1
			|| pos.z == 0 || pos.z == Self::SIZE_I-1
		{
			return true;
		}
		false
	}

	/// Returns a reference to the mesh if it exists
	#[inline] pub const fn mesh(&self) -> Option<&GeometryBuffer> { self.mesh.as_ref() }
	#[inline] pub fn set_mesh(&mut self, gb: Option<GeometryBuffer>) { self.mesh = gb; }
	
	/// Returns a reference to the bind group if it exists
	#[inline] pub const fn bind_group(&self) -> Option<&wgpu::BindGroup> { self.bind_group.as_ref() }
	#[inline] pub fn set_bind_group(&mut self, bg: Option<wgpu::BindGroup>) { self.bind_group = bg; }

	#[inline] pub const fn finished_gen(&self) -> bool { self.finished_gen }

	#[inline] pub const fn storage(&self) -> &BlockStorage { &self.storage }
	#[inline] pub const fn storage_mut(&mut self) -> &mut BlockStorage { &mut self.storage }

	#[inline] pub const fn entities(&self) -> &EntityStorage { &self.entities }
	#[inline] pub const fn entities_mut(&mut self) -> &mut EntityStorage { &mut self.entities }


	#[inline] pub fn optimize_storage(&mut self) { self.storage.optimize(); }
	#[inline] pub fn storage_info(&self) -> (usize, &'static str) { self.storage.memory_usage() }
}
