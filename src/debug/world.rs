
#[cfg(test)]
mod tests {
	use crate::{
		block::{
			main::{Block, StorageType, BlockStorage, Chunk, Material},
			math::{BlockPosition, BlockRotation, ChunkCoord},
		},
		hs::binary::BinarySerializable,
		world::main::World,
	};
	use glam::IVec3;

	#[test]
	fn basic_block_operations() {
		let mut chunk = Chunk::empty();
		let pos = BlockPosition::new(1, 2, 3);
		let idx: usize = pos.into();

		// Test setting and getting a block
		let block = block(2);
		chunk.set_block(idx, block);
		assert_eq!(chunk.get_block(idx), block);
		assert!(chunk.dirty);

		// Test block rotation
		let rotated_block = Block::from(Material(2), BlockRotation::XminusYplus);
		chunk.set_block(idx, rotated_block);
		assert_eq!(chunk.get_block(idx).get_rotation(), BlockRotation::XminusYplus);
	}

	#[test]
	fn border_block_detection() {
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
	fn palette_compaction() {
		let mut chunk = Chunk::empty();
		
		// Add several blocks of the same type
		for i in 0..10 {
			let pos = BlockPosition::new(i % 16, (i / 16) % 16, 0);
			chunk.set_block(pos.into(), block(2));
		}
		
		// Should still be in Compact storage
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));

		// Optimize storage (should stay Compact)
		chunk.optimize_storage();
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));

		// Set all blocks to the same type
		for i in 0..Chunk::VOLUME {
			chunk.set_block(i, block(2));
		}
		
		// Optimize should convert to Uniform
		chunk.optimize_storage();
		assert!(matches!(chunk.storage, BlockStorage::Uniform { .. }));
	}

	#[test]
	fn rle_compression() {
		// Create a pattern that will compress well with RLE
		let block_a = block(1);
		let block_b = block(3);
		
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
	fn rle_edge_cases() {
		// Test empty chunk
		let empty_chunk = Chunk::empty();
		assert!(empty_chunk.storage.to_rle().is_none());
		
		// Test uniform chunk
		let uniform_chunk = Chunk::new(2);
		assert!(uniform_chunk.storage.to_rle().is_none());
		
		// Test worst-case scenario for RLE (no compression)
		let mut worst_case = Chunk::empty();
		for i in 0..Chunk::VOLUME {
			worst_case.set_block(i, block(2 + (i % 2) as u16));
		}
		assert!(worst_case.storage.to_rle().is_none());
	}

	#[test]
	fn chunk_generation() {
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
	fn storage_transitions() {
		let mut chunk = Chunk::empty();
		
		// 1. Initial state should be Uniform storage with air blocks (or default)
		assert!(matches!(chunk.storage, BlockStorage::Uniform { .. }));
		assert_eq!(chunk.get_block(BlockPosition::ZERO.into()), block(1));
		assert_eq!(chunk.get_block(BlockPosition::CORNER.into()), block(1));

		// 2. Setting one non-default block should transition to Compact storage
		let pos1 = BlockPosition::new(1, 1, 1);
		let block1 = block(2);
		chunk.set_block(pos1.into(), block1);
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));
		assert_eq!(chunk.get_block(pos1.into()), block1);
		
		// Verify all other blocks are still default
		assert_eq!(chunk.get_block(BlockPosition::ZERO.into()), block(1));
		assert_eq!(chunk.get_block(BlockPosition::CORNER.into()), block(1));

		// 3. Adding a second block type should stay in Compact storage
		let pos2 = BlockPosition::new(2, 2, 2);
		let block2 = block(3);
		chunk.set_block(pos2.into(), block2);
		assert!(matches!(chunk.storage, BlockStorage::Compact { .. }));
		assert_eq!(chunk.get_block(pos2.into()), block2);

		// 4. Adding enough unique blocks should transition to Sparse storage
		// We'll add 20 unique block types (including the existing 2)
		for i in 0..18 {
			let pos = BlockPosition::new(i % 16, (i / 16) % 16, 0);
			let block = block(4 + i as u16);
			chunk.set_block(pos.into(), block);
			assert_eq!(chunk.get_block(pos.into()), block);
		}
		assert!(matches!(chunk.storage, BlockStorage::Sparse { .. }));

		// 5. Verify we can still access all previously set blocks
		assert_eq!(chunk.get_block(pos1.into()), block1);
		assert_eq!(chunk.get_block(pos2.into()), block2);
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
		let block_a = block(1);
		let block_b = block(2);
		let block_c = block(3);
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



	fn block(id: u16) -> Block {
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
	fn empty_storage() {
		let storage = BlockStorage::empty();
		assert_eq!(storage.to_type(), StorageType::Uniform);
		assert_eq!(storage.get(0), Block::default());
		assert_eq!(storage.get(1234), Block::default());
		assert_eq!(storage.get(4095), Block::default());
	}

	#[test]
	fn uniform_storage() {
		let stone = block(1);
		let storage = BlockStorage::uniform(stone);
		
		assert_eq!(storage.to_type(), StorageType::Uniform);
		assert_eq!(storage.get(0), stone);
		assert_eq!(storage.get(4095), stone);
		
		let mut storage = storage;
		storage.set(0, block(2)); // First modification should transition
		
		// Should have transitioned to Compact storage
		assert_eq!(storage.to_type(), StorageType::Compact);
		assert_eq!(storage.palette().len(), 2);
	}

	#[test]
	fn compact_storage_transitions() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Add 15 distinct blocks (total 16, which is Compact's max)
		for i in 0..15 {
			storage.set(i, block(2 + i as u16));
		}
		
		assert_eq!(storage.to_type(), StorageType::Compact);
		assert_eq!(storage.palette().len(), 16);
		
		// Adding one more should transition to Sparse
		storage.set(16, block(100));
		assert_eq!(storage.to_type(), StorageType::Sparse);
		assert_eq!(storage.palette().len(), 17);
	}

	#[test]
	fn sparse_storage_transitions() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Force transition to Sparse
		for i in 0..256 {
			storage.set(i, block(i as u16 + 1));
		}
		
		assert_eq!(storage.to_type(), StorageType::Sparse);
		assert_eq!(storage.palette().len(), 256);
		
		// Adding one more should transition to Giant
		storage.set(256, block(300));
		assert_eq!(storage.to_type(), StorageType::Giant);
		assert_eq!(storage.palette().len(), 257);
	}

	#[test]
	fn giant_storage_transitions() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Force transition to Giant
		for i in 0..4096 {
			storage.set(i, block(i as u16 / 2 + 1));
		}
		
		assert_eq!(storage.palette().len(), 2048);
		assert_eq!(storage.to_type(), StorageType::Giant);

		// Adding one more should transition to Zigzag
		for i in 0..4096 {
			storage.set(i, block(i as u16 + 1));
		}
		assert_eq!(storage.palette().len(), 4096); 
		assert_eq!(storage.to_type(), StorageType::Giant);
		// Zigzag only occurs at over 4K what i can not do in 16^3 chunks
	}

	#[test]
	fn zigzag_storage() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Force transition to Zigzag
		for i in 0..4096 {
			storage.set(i, block(i as u16 + 1));
		}
		
		// Verify all blocks
		for i in 0..4096 {
			assert_eq!(storage.get(i), block(i as u16 + 1));
		}
		
		// Modify some blocks
		storage.set(0, block(5000));
		storage.set(2048, block(6000));
		assert_eq!(storage.get(0), block(5000));
		assert_eq!(storage.get(2048), block(6000));
	}

	#[test]
	fn optimize_uniform() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Set all blocks to the same value
		for i in 0..4096 {
			storage.set(i, block(1));
		}
		
		storage.optimize();
		assert_eq!(storage.to_type(), StorageType::Uniform);
	}

	#[test]
	fn optimize_sparse_to_compact() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Create a pattern that uses only 16 distinct blocks
		for i in 0..4096 {
			storage.set(i, block((i % 16) as u16 + 1));
		}
		
		// Force to Sparse first
		for i in 0..4096 {
			storage.set(i, block((i % 32) as u16 + 1));
		}
		assert_eq!(storage.to_type(), StorageType::Sparse);
		for i in 0..4096 {
			storage.set(i, block((i % 16) as u16 + 1));
		}
		
		storage.optimize();
		assert_eq!(storage.to_type(), StorageType::Compact);
		assert_eq!(storage.palette().len(), 16);
	}

	#[test]
	fn optimize_to_uniform_after_clearing() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Create diverse storage
		for i in 0..4096 {
			storage.set(i, block((i % 20) as u16 + 1));
		}
		assert_ne!(storage.to_type(), StorageType::Uniform);
		
		// Set all blocks back to air
		for i in 0..4096 {
			storage.set(i, Block::default());
		}
		
		storage.optimize();
		assert_eq!(storage.to_type(), StorageType::Uniform);
		assert_eq!(storage.get(0), Block::default());
	}

	#[test]
	fn palette_management() {
		let mut storage = BlockStorage::uniform(block(1));
		
		// Add 5 distinct blocks
		for i in 0..5 {
			storage.set(i * 100, block(i as u16 + 2));
		}
		
		match &storage {
			BlockStorage::Compact { palette, .. } => {
				assert_eq!(palette.len(), 6); // original + 5 new
				assert!(palette.contains(&block(1)));
				for i in 0..5 {
					assert!(palette.contains(&block(i as u16 + 2)));
				}
			},
			_ => panic!("Expected Compact storage"),
		}
	}

	#[test]
	fn memory_usage_reporting() {
		let uniform = BlockStorage::uniform(block(1));
		let (size, name) = uniform.memory_usage();
		assert_eq!(name, "Uniform");
		assert_eq!(size, std::mem::size_of::<Block>());
		
		let mut compact = BlockStorage::uniform(block(1));
		compact.set(0, block(2));
		let (size, name) = compact.memory_usage();
		assert_eq!(name, "Compact");
		assert!(size > 2048); // Indices size
		
		let mut sparse = BlockStorage::uniform(block(1));
		for i in 0..20 {
			sparse.set(i, block(i as u16 + 1));
		}
		let (size, name) = sparse.memory_usage();
		assert_eq!(name, "Sparse");
		assert!(size > 4096); // Indices size
		
		let mut giant = BlockStorage::uniform(block(1));
		for i in 0..300 {
			giant.set(i, block(i as u16 + 1));
		}
		let (size, name) = giant.memory_usage();
		assert_eq!(name, "Giant");
		assert!(size > 4096 * 3 / 2); // Indices size
		
		let mut zigzag = BlockStorage::uniform(block(1));
		for i in 0..4096 {
			zigzag.set(i, block(i as u16 + 5));
		}
		let (size, name) = zigzag.memory_usage();
		assert_eq!(name, "Zigzag"); //propably will not turn into zigzag ...
		assert_eq!(size, 4096 * std::mem::size_of::<Block>());
	}

	#[test]
	fn edge_cases() {
		// Test first and last positions
		let mut storage = BlockStorage::uniform(block(1));
		storage.set(0, block(2));
		storage.set(4095, block(3));
		
		assert_eq!(storage.get(0), block(2));
		assert_eq!(storage.get(4095), block(3));
		
		// Test setting same block doesn't change storage
		let mut storage = BlockStorage::uniform(block(1));
		let before = storage.clone();
		storage.set(0, block(1));
		assert_eq!(storage, before);
		
		// Test palette full fallback
		let mut storage = BlockStorage::uniform(block(1));
		for i in 0..4096 {
			storage.set(i, block(i as u16 + 1));
		}
		// Palette should be full now
		assert_eq!(storage.to_type(), StorageType::Giant);
	}

	#[test]
	fn compact_index_packing() {
		let mut indices = Box::new([0u8; Chunk::VOLUME/2]);
		
		// Test all positions
		for i in 0..Chunk::VOLUME {
			let value = (i % 16) as u8;
			BlockStorage::set_compact_index(&mut indices, i, value);
			assert_eq!(BlockStorage::get_compact_index(&*indices, i), value);
		}
		
		// Test adjacent positions don't interfere
		BlockStorage::set_compact_index(&mut *indices, 0, 0x0A);
		BlockStorage::set_compact_index(&mut *indices, 1, 0x0B);
		assert_eq!(indices[0], 0xBA);
	}

	#[test]
	fn giant_index_packing() {
		let mut indices = Box::new([0u8; Chunk::VOLUME * 3 / 2]);
		
		// Test all positions with 12-bit values
		for i in 0..Chunk::VOLUME {
			let value = (i % 4096) as u16;
			BlockStorage::set_giant_index(&mut *indices, i, value);
			assert_eq!(BlockStorage::get_giant_index(&*indices, i), value);
		}
		
		// Test adjacent positions don't interfere
		BlockStorage::set_giant_index(&mut *indices, 0, 0x123);
		BlockStorage::set_giant_index(&mut *indices, 1, 0x456);
		assert_eq!(BlockStorage::get_giant_index(&*indices, 0), 0x123);
		assert_eq!(BlockStorage::get_giant_index(&*indices, 1), 0x456);
	}

	#[test]
	fn random_access_pattern() {
		let mut storage = BlockStorage::uniform(block(1));
		let mut expected = [Block::default(); Chunk::VOLUME];
		expected.fill(block(1));
		
		// Randomly set blocks and verify
		let mut rng = crate::hs::math::Rand::from_time();
		for _ in 0..1000 {
			let pos = rng.range(0..Chunk::VOLUME);
			let block = block(rng.range_inc(1..=100) as u16);
			
			storage.set(pos, block);
			expected[pos] = block;
		}
		
		// Verify all positions
		for i in 0..Chunk::VOLUME {
			assert_eq!(storage.get(i), expected[i]);
		}
	}
}