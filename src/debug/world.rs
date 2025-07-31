
#[cfg(test)]
mod tests {
	use crate::{
		block::{
			main::{Block, BlockStorage, Chunk, Material},
			math::{BlockPosition, BlockRotation, ChunkCoord},
		},
		hs::binary::BinarySerializable,
		world::main::World,
	};
	use glam::IVec3;

	#[test]
	fn test_basic_block_operations() {
		let mut chunk = Chunk::empty();
		let pos = BlockPosition::new(1, 2, 3);
		let idx: usize = pos.into();

		// Test setting and getting a block
		let block = test_block(2);
		chunk.set_block(idx, block);
		assert_eq!(chunk.get_block(idx), block);
		assert!(chunk.dirty);

		// Test block rotation
		let rotated_block = Block::from(Material(2), BlockRotation::XminusYplus);
		chunk.set_block(idx, rotated_block);
		assert_eq!(chunk.get_block(idx).get_rotation(), BlockRotation::XminusYplus);
	}

	#[test]
	fn test_border_block_detection() {
		let chunk = Chunk::empty();

		// Test edge cases
		assert!(chunk.is_border_block(IVec3::new(0, 5, 5)));
		assert!(chunk.is_border_block(IVec3::new(15, 5, 5)));
		assert!(chunk.is_border_block(IVec3::new(5, 0, 5)));
		assert!(chunk.is_border_block(IVec3::new(5, 15, 5)));
		assert!(chunk.is_border_block(IVec3::new(5, 5, 0)));
		assert!(chunk.is_border_block(IVec3::new(5, 5, 15)));

		// Test non-border cases
		assert!(!chunk.is_border_block(IVec3::new(1, 1, 1)));
		assert!(!chunk.is_border_block(IVec3::new(14, 14, 14)));
	}

	#[test]
	fn test_palette_compaction() {
		let mut chunk = Chunk::empty();
		
		// Add several blocks of the same type
		for i in 0..10 {
			let pos = BlockPosition::new(i % 16, (i / 16) % 16, 0);
			chunk.set_block(pos.into(), test_block(2));
		}
		
		// Should still be in Compact storage
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));

		// Optimize storage (should stay Compact)
		chunk.optimize_storage();
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));

		// Set all blocks to the same type
		for i in 0..Chunk::VOLUME {
			chunk.set_block(i, test_block(2));
		}
		
		// Optimize should convert to Uniform
		chunk.optimize_storage();
		assert!(matches!(chunk.storage, BlockStorage::Uniform { .. }));
	}

	#[test]
	fn test_rle_compression() {
		// Create a pattern that will compress well with RLE
		let block_a = test_block(1);
		let block_b = test_block(3);
		
		let mut chunk = rle_chunk(block_a, block_b);
		
		// Convert to RLE
		if let Some(rle_storage) = chunk.storage.to_rle() {
			chunk.storage = rle_storage;
			assert!(matches!(chunk.storage, BlockStorage::Rle { .. }));
			
			// Verify we can convert back
			if let Some(original_storage) = chunk.storage.from_rle() {
				chunk.storage = original_storage;
				assert!(!matches!(chunk.storage, BlockStorage::Rle { .. }));
				
				// Verify all blocks are preserved
				for y in 0..Chunk::SIZE {
					for z in 0..Chunk::SIZE {
						for x in 0..Chunk::SIZE {
							let expected = if (x + y + z) % 2 == 0 { block_a } else { block_b };
							let pos = BlockPosition::from((x, y, z));
							assert_eq!(chunk.get_block(pos.into()), expected);
						}
					}
				}
			}
		}
	}

	#[test]
	fn test_rle_edge_cases() {
		// Test empty chunk
		let empty_chunk = Chunk::empty();
		assert!(empty_chunk.storage.to_rle().is_none());
		
		// Test uniform chunk
		let uniform_chunk = Chunk::new(2);
		assert!(uniform_chunk.storage.to_rle().is_none());
		
		// Test worst-case scenario for RLE (no compression)
		let mut worst_case = Chunk::empty();
		for i in 0..Chunk::VOLUME {
			worst_case.set_block(i, test_block(2 + (i % 2) as u16));
		}
		assert!(worst_case.storage.to_rle().is_none());
	}

	#[test]
	fn test_chunk_generation() {
		let coord = ChunkCoord::new(0, 0, 0);
		let chunk = Chunk::generate(coord, 12345).unwrap();
		
		// Verify chunk isn't empty
		assert!(!chunk.is_empty());
		
		// Verify some basic properties
		let mut air_count = 0;
		let mut solid_count = 0;
		
		for i in 0..Chunk::VOLUME {
			if chunk.get_block(i).is_empty() {
				air_count += 1;
			} else {
				solid_count += 1;
			}
		}
		
		assert!(solid_count > 0);
		assert!(air_count > 0);
	}

	#[test]
	fn test_storage_transitions() {
		let mut chunk = Chunk::empty();
		
		// 1. Initial state should be Uniform storage with air blocks (or default)
		assert!(matches!(chunk.storage, BlockStorage::Uniform { .. }));
		assert_eq!(chunk.get_block(BlockPosition::ZERO.into()), test_block(1));
		assert_eq!(chunk.get_block(BlockPosition::CORNER.into()), test_block(1));

		// 2. Setting one non-default block should transition to Compact storage
		let pos1 = BlockPosition::new(1, 1, 1);
		let block1 = test_block(2);
		chunk.set_block(pos1.into(), block1);
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));
		assert_eq!(chunk.get_block(pos1.into()), block1);
		
		// Verify all other blocks are still default
		assert_eq!(chunk.get_block(BlockPosition::ZERO.into()), test_block(1));
		assert_eq!(chunk.get_block(BlockPosition::CORNER.into()), test_block(1));

		// 3. Adding a second block type should stay in Compact storage
		let pos2 = BlockPosition::new(2, 2, 2);
		let block2 = test_block(3);
		chunk.set_block(pos2.into(), block2);
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));
		assert_eq!(chunk.get_block(pos2.into()), block2);

		// 4. Adding enough unique blocks should transition to Sparse storage
		// We'll add 20 unique block types (including the existing 2)
		for i in 0..18 {
			let pos = BlockPosition::new(i % 16, (i / 16) % 16, 0);
			let block = test_block(4 + i as u16);
			chunk.set_block(pos.into(), block);
			assert_eq!(chunk.get_block(pos.into()), block);
		}
		assert!(matches!(chunk.storage, BlockStorage::Sparse { .. }));

		// 5. Verify we can still access all previously set blocks
		assert_eq!(chunk.get_block(pos1.into()), block1);
		assert_eq!(chunk.get_block(pos2.into()), block2);
	}

	fn test_block(id: u16) -> Block {
		Block::new(Material(id))
	}
	fn rle_chunk(bc: Block, bck:Block) -> Chunk {
		let mut chunk = Chunk::empty();
		// Create stripes pattern
		for y in 0..Chunk::SIZE {
			for z in 0..Chunk::SIZE {
				for x in 0..Chunk::SIZE {
					let block = if (x + y + z) % 2 == 0 { bc } else { bck };
					let pos = BlockPosition::from((x, y, z));
					chunk.set_block(pos.into(), block);
				}
			}
		}
		chunk
	}

	#[test]
	fn save_load_world() {
		let mut world = World::empty();
		let coord = ChunkCoord::new(1, 2, 3);
		let chunk = Chunk::new(1u16);
		world.chunks.insert(coord, chunk);
		
		let world_data = world.to_binary();
		let restored_world = World::from_binary(&world_data).unwrap();
		
		assert_eq!(world.chunks.len(), restored_world.chunks.len());
		let restored_chunk = restored_world.chunks.get(&coord).unwrap();
		assert_eq!(&Chunk::new(1u16), restored_chunk);
		assert_eq!(world.chunks, restored_world.chunks);
	}
	
	#[test]
	fn save_load_complex_world() {
		let mut world = World::empty();
		let coord = ChunkCoord::new(1, 2, 3);

		let simple_chunk = Chunk::new(5u16);
		let block_a = test_block(1);
		let block_b = test_block(2);
		let block_c = test_block(3);
		let rle_chunk_air = rle_chunk(block_a, block_b);
		let rle_chunk = rle_chunk(block_b, block_c);

		world.chunks.insert(coord, rle_chunk_air.clone());
		world.chunks.insert(coord.offset(0,1,0), simple_chunk.clone());
		world.chunks.insert(coord.offset(0,10,-2), rle_chunk.clone());

		let world_data = world.to_binary();
		let restored_world = World::from_binary(&world_data).unwrap();
		
		assert_eq!(world.chunks.len(), restored_world.chunks.len());
		assert_eq!(world.chunks, restored_world.chunks);

		assert_eq!(&rle_chunk_air, restored_world.chunks.get(&coord).unwrap());
		assert_eq!(&simple_chunk, restored_world.chunks.get(&coord.offset(0,1,0)).unwrap());
		assert_eq!(&rle_chunk, restored_world.chunks.get(&coord.offset(0,10,-2)).unwrap());
	}

	#[test]
	fn save_load_single_chunk() {		
		let chunk = Chunk::new(10u16);
		let restored_chunk = Chunk::from_binary(&chunk.to_binary()).unwrap();

		let coord = ChunkCoord::new(1, 2, 3);
		let restored_coord = ChunkCoord::from_binary(&coord.to_binary()).unwrap();
		
		assert_eq!(coord, restored_coord);
		assert_eq!(chunk, restored_chunk);
	}
}