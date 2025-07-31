#[cfg(test)]
use crate::world::manager::{get_save_path, load_entire_world, save_entire_world};
#[cfg(test)]
use crate::world::main::World;
#[cfg(test)]
use crate::block::math::{BlockRotation, ChunkCoord};
#[cfg(test)]
use crate::block::main::{self, Block, Chunk, BlockStorage};
#[cfg(test)]
use crate::ext::ptr;
#[cfg(test)]
use std::io::{Read, Write};
#[cfg(test)]
use crate::game::state;
#[cfg(test)]
use std::fs::{self, File};
#[cfg(test)]
use crate::hs::binary::BinarySerializable;


#[test]
fn save_load_single_chunk() {
	let mut world = World::empty();
	let coord = ChunkCoord::new(1, 2, 3);
	
	let chunk = Chunk::new(1u16);
	world.chunks.insert(coord.into(), chunk);
	
	let chunk_data = world.get_chunk(coord).unwrap().to_binary();
	let restored_chunk = Chunk::from_binary(&chunk_data).unwrap();

	let coord_data = coord.to_binary();
	let restored_coord = ChunkCoord::from_binary(&coord_data).unwrap();
	
	assert_eq!(coord, restored_coord);
	let original_chunk = world.chunks.get(&coord.into()).unwrap();
	assert_eq!(original_chunk, &restored_chunk);
}

