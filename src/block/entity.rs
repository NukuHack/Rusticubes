// Block entity
use crate::item::inventory::ItemContainer;
use crate::block::math::LocalPos;
use crate::block::main::Chunk;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use ahash::AHasher;

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

#[derive(Clone, PartialEq, Debug)]
pub enum EntityStorage {
	Empty, // 1 byte
	Sparse(FastMap<LocalPos, ItemContainer>), // small storage
	Dense(Box<[Option<ItemContainer>; Chunk::VOLUME]>), // For >25% density
}

impl EntityStorage {
	/// Creates a new empty storage
	pub fn default() -> Self { EntityStorage::Empty }

	/// Adds an entity at the given position
	pub fn add(&mut self, pos: LocalPos, entity: ItemContainer) {
		match self {
			EntityStorage::Empty => {
				let mut map = FastMap::with_capacity_and_hasher(5, BuildHasherDefault::<AHasher>::default());
				map.insert(pos, entity);
				*self = EntityStorage::Sparse(map);
			},
			EntityStorage::Sparse(map) => {
				map.insert(pos, entity);
				// Auto-upgrade to dense if density exceeds threshold
				if map.len() > Chunk::VOLUME / 4 {
					self.upgrade_to_dense();
				}
			},
			EntityStorage::Dense(array) => {
				array[pos.index() as usize].replace(entity);
			},
		}
	}

	/// Removes an entity at the given position
	pub fn remove(&mut self, pos: LocalPos) -> Option<ItemContainer> {
		match self {
			EntityStorage::Empty => None,
			EntityStorage::Sparse(map) => {
				let removed = map.remove(&pos);
				if map.is_empty() {
					*self = EntityStorage::Empty;
				}
				removed
			},
			EntityStorage::Dense(array) => {
				let removed = array[pos.index() as usize].take();
				// Downgrade to sparse if density is low enough
				if array.len() < Chunk::VOLUME / 8 {
					self.downgrade_to_sparse();
				}
				removed
			},
		}
	}

	/// Gets a reference to an entity at the given position
	pub fn get(&self, pos: LocalPos) -> Option<&ItemContainer> {
		match self {
			EntityStorage::Empty => None,
			EntityStorage::Sparse(map) => map.get(&pos),
			EntityStorage::Dense(array) => array[pos.index() as usize].as_ref(),
		}
	}
	/// Gets a mutable reference to an entity at the given position
	pub fn get_mut(&mut self, pos: LocalPos) -> Option<&mut ItemContainer> {
		match self {
			EntityStorage::Empty => None,
			EntityStorage::Sparse(map) => map.get_mut(&pos),
			EntityStorage::Dense(array) => array[pos.index() as usize].as_mut(),
		}
	}

	/// Checks if the storage contains an entity at the given position
	pub fn contains(&self, pos: LocalPos) -> bool {
		match self {
			EntityStorage::Empty => false,
			EntityStorage::Sparse(map) => map.contains_key(&pos),
			EntityStorage::Dense(array) => array[pos.index() as usize].is_some(),
		}
	}

	/// Returns the number of entities stored
	pub fn len(&self) -> usize {
		match self {
			EntityStorage::Empty => 0,
			EntityStorage::Sparse(map) => map.len(),
			EntityStorage::Dense(array) => array.iter().filter(|e| e.is_some()).count(),
		}
	}
	/// Checks if the storage is empty
	pub fn is_empty(&self) -> bool {
		match self {
			EntityStorage::Empty => true,
			EntityStorage::Sparse(map) => map.is_empty(),
			EntityStorage::Dense(array) => array.iter().all(|e| e.is_none()),
		}
	}

	/// Clears all entities from storage
	pub fn clear(&mut self) { *self = EntityStorage::Empty; }

	/// Optimizes the storage by choosing the best representation
	pub fn optimize(&mut self) {
		match self {
			EntityStorage::Sparse(map) => {
				if map.is_empty() {
					*self = EntityStorage::Empty;
				} else if map.len() > Chunk::VOLUME / 4 {
					self.upgrade_to_dense();
				}
			},
			EntityStorage::Dense(_array) => {
				if self.is_empty() {
					*self = EntityStorage::Empty;
				} else if self.len() < Chunk::VOLUME / 8 {
					self.downgrade_to_sparse();
				}
			},
			EntityStorage::Empty => {} // Already optimal
		}
	}

	/// Iterates over all entities with their positions
	pub fn iter(&self) -> Box<dyn Iterator<Item = (LocalPos, &ItemContainer)> + '_> {
		match self {
			EntityStorage::Empty => Box::new(std::iter::empty()),
			EntityStorage::Sparse(map) => Box::new(map.iter().map(|(pos, entity)| (*pos, entity))),
			EntityStorage::Dense(array) => Box::new(
				array.iter()
					.enumerate()
					.filter_map(|(i, entity)| {
						entity.as_ref().map(|e| (LocalPos::from_index(i as u16), e))
					})
			),
		}
	}
	/// Iterates over all entities mutably with their positions
	pub fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (LocalPos, &mut ItemContainer)> + '_> {
		match self {
			EntityStorage::Empty => Box::new(std::iter::empty()),
			EntityStorage::Sparse(map) => Box::new(map.iter_mut().map(|(pos, entity)| (*pos, entity))),
			EntityStorage::Dense(array) => Box::new(
				array.iter_mut()
					.enumerate()
					.filter_map(|(i, entity)| {
						entity.as_mut().map(|e| (LocalPos::from_index(i as u16), e))
					})
			),
		}
	}

	/// Returns the memory usage of the storage in bytes (approximate)
	pub fn memory_usage(&self) -> usize {
		match self {
			EntityStorage::Empty => std::mem::size_of::<Self>(),
			EntityStorage::Sparse(map) => {
				std::mem::size_of::<Self>() + 
				map.capacity() * (std::mem::size_of::<LocalPos>() + std::mem::size_of::<ItemContainer>())
			}
			EntityStorage::Dense(_) => {
				std::mem::size_of::<Self>() + 
				Chunk::VOLUME * std::mem::size_of::<Option<ItemContainer>>()
			}
		}
	}
	// Private helper methods
	fn upgrade_to_dense(&mut self) {
		if let EntityStorage::Sparse(map) = self {
			let mut dense = Box::new([const { None }; Chunk::VOLUME]);
			for (pos, entity) in map.drain() {
				dense[pos.index() as usize] = Some(entity);
			}
			*self = EntityStorage::Dense(dense);
		}
	}
	fn downgrade_to_sparse(&mut self) {
		if let EntityStorage::Dense(array) = self {
			let mut map = FastMap::with_capacity_and_hasher(array.len() / 2, BuildHasherDefault::<AHasher>::default());
			for (i, entity) in array.iter().enumerate() {
				if let Some(entity) = entity {
					map.insert(LocalPos::from_index(i as u16), entity.clone());
				}
			}
			*self = if map.is_empty() {
				EntityStorage::Empty
			} else {
				EntityStorage::Sparse(map)
			};
		}
	}
}

// Small inline functions in Chunk that delegate to EntityStorage
impl Chunk {
	/// Adds an entity at the given position
	#[inline]
	pub fn add_entity(&mut self, pos: LocalPos, entity: ItemContainer) {
		self.entities_mut().add(pos, entity);
	}

	/// Removes an entity at the given position
	#[inline]
	pub fn remove_entity(&mut self, pos: LocalPos) -> Option<ItemContainer> {
		self.entities_mut().remove(pos)
	}

	/// Gets a reference to an entity at the given position
	#[inline]
	pub fn get_entity(&self, pos: LocalPos) -> Option<&ItemContainer> {
		self.entities().get(pos)
	}

	/// Gets a mutable reference to an entity at the given position
	#[inline]
	pub fn get_entity_mut(&mut self, pos: LocalPos) -> Option<&mut ItemContainer> {
		self.entities_mut().get_mut(pos)
	}

	/// Checks if an entity exists at the given position
	#[inline]
	pub fn has_entity(&self, pos: LocalPos) -> bool {
		self.entities().contains(pos)
	}

	/// Returns the number of entities in the chunk
	#[inline]
	pub fn entity_count(&self) -> usize {
		self.entities().len()
	}
}
