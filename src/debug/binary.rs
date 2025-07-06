

#[cfg(test)]
use crate::world::manager::save_entire_world;
#[cfg(test)]
use crate::world::manager::load_entire_world;
#[cfg(test)]
use crate::world::main::World;
#[cfg(test)]
use crate::block::math::{BlockRotation, ChunkCoord};
#[cfg(test)]
use crate::block::main::{Block, Chunk, BlockStorage};
#[cfg(test)]
use crate::config;
#[cfg(test)]
use std::io::{Read, Write};
#[cfg(test)]
use crate::game_state;
#[cfg(test)]
use std::fs::{self, File};

#[test]
fn chunk_coord_conversion() {
    let original = ChunkCoord::new(1, 2, 3);
    let bytes = original.to_bytes();
    let restored = ChunkCoord::from_bytes(bytes);
    assert_eq!(original, restored);
}

#[test]
fn chunk_conversion() {
    let original = Chunk::new();
    let bytes = original.to_binary();
    let restored = Chunk::from_binary(&bytes);
    assert_eq!(original, restored.expect("test should work"));
}

#[test]
fn block_rotation_conversion() {
    // Test all possible rotations
    for i in 0..24 {
        let rotation = BlockRotation::from_byte(i).unwrap();
        assert_eq!(rotation.to_byte(), i);
    }
}

#[test]
fn block_rotation_invalid() {
    assert!(BlockRotation::from_byte(24).is_none());
    assert!(BlockRotation::from_byte(255).is_none());
}

#[test]
fn block_serialization() {
    let test_blocks = [
        Block::None,
        Block::Simple(42, BlockRotation::XplusYplus),
        Block::Simple(65535, BlockRotation::ZminusYminus),
        Block::Marching(123, 456789),
    ];
    
    for block in test_blocks {
        let binary = block.to_binary();
        let restored = Block::from_binary(&binary).unwrap();
        assert_eq!(block, restored);
    }
}

#[test]
fn block_no_data() {
    // Too short for any block
    assert!(Block::from_binary(&[]).is_none());
    // Invalid block type
    assert!(Block::from_binary(&[3]).is_none());
}
#[test]
fn block_missing_data() {
    // Simple block missing data
    assert!(Block::from_binary(&[1]).is_none());
    assert!(Block::from_binary(&[1, 0]).is_none());
    assert!(Block::from_binary(&[1, 0, 0]).is_none());
}
#[test]
fn block_invalid_data() {
    // Marching block missing data
    assert!(Block::from_binary(&[2]).is_none());
    assert!(Block::from_binary(&[2, 0]).is_none());
    assert!(Block::from_binary(&[2, 0, 0, 0]).is_none());
}

#[test]
fn block_binary_size() {
    assert_eq!(Block::None.binary_size(), 1);
    assert_eq!(Block::Simple(0, BlockRotation::XplusYplus).binary_size(), 4);
    assert_eq!(Block::Marching(0, 0).binary_size(), 7);
}

#[test]
fn empty_chunk_serialization() {
    let original = Chunk::new();
    let binary = original.to_binary();
    let restored = Chunk::from_binary(&binary).unwrap();
    assert_eq!(original.palette, restored.palette);
    assert_eq!(original.storage, restored.storage);
}

#[test]
fn uniform_chunk_serialization() {
    let mut chunk = Chunk::new();
    chunk.palette = vec![Block::Simple(1, BlockRotation::XplusYplus)];
    chunk.storage = BlockStorage::Uniform(0);
    
    let binary = chunk.to_binary();
    let restored = Chunk::from_binary(&binary).unwrap();
    assert_eq!(chunk.palette, restored.palette);
    assert_eq!(chunk.storage, restored.storage);
}

#[test]
fn sparse_chunk_serialization() {
    let mut chunk = Chunk::new();
    chunk.palette = vec![
        Block::None,
        Block::Simple(1, BlockRotation::XplusYplus),
        Block::Marching(2, 12345),
    ];
    
    let mut indices = Box::new([0; 4096]);
    for i in 0..4096 {
        indices[i] = (i % 3) as u8;
    }
    chunk.storage = BlockStorage::Sparse(indices);
    
    let binary = chunk.to_binary();
    let restored = Chunk::from_binary(&binary).unwrap();
    assert_eq!(chunk.palette, restored.palette);
    assert_eq!(chunk.storage, restored.storage);
}

#[test]
fn chunk_no_data() {
    // Empty data
    assert!(Chunk::from_binary(&[]).is_none());
    
    // Missing palette entries
    assert!(Chunk::from_binary(&[1]).is_none()); // Says palette has 1 entry but no data
}
#[test]
fn chunk_invalid_data() {
    // Invalid storage type
    let mut data = vec![1]; // 1 palette entry
    data.extend(Block::None.to_binary()); // Add the palette entry
    data.push(2); // Invalid storage type
    assert!(Chunk::from_binary(&data).is_none());
}
#[test]
fn chunk_missing_data() { 
    // Sparse storage missing data
    let mut data = vec![1]; // 1 palette entry
    data.extend(Block::None.to_binary()); // Add the palette entry
    data.push(1); // Sparse storage type
    assert!(Chunk::from_binary(&data).is_none()); // Missing indices
}

#[test]
fn chunk_binary_size() {
    let mut chunk = Chunk::new();
    assert_eq!(chunk.binary_size(), chunk.to_binary().len());
    
    chunk.palette = vec![Block::None];
    assert_eq!(chunk.binary_size(), chunk.to_binary().len());
    
    chunk.storage = BlockStorage::Uniform(0);
    assert_eq!(chunk.binary_size(), chunk.to_binary().len());
    
    chunk.palette = vec![Block::None, Block::Simple(1, BlockRotation::XplusYplus)];
    chunk.storage = BlockStorage::Sparse(Box::new([0; 4096]));
    assert_eq!(chunk.binary_size(), chunk.to_binary().len());
}

#[test]
fn empty_world_serialization() {
    let original = World::empty();
    let binary = original.to_binary();
    let restored = World::from_binary(&binary).unwrap();
    assert_eq!(original.chunks.len(), restored.chunks.len());
}

#[test]
fn world_with_chunks_serialization() {
    let mut world = World::empty();
    
    // Add some chunks
    let mut chunk1 = Chunk::new();
    chunk1.palette = vec![Block::Simple(1, BlockRotation::XplusYplus)];
    chunk1.storage = BlockStorage::Uniform(0);
    world.chunks.insert(ChunkCoord::new(0, 0, 0).into(), chunk1);
    
    let mut chunk2 = Chunk::new();
    chunk2.palette = vec![Block::None, Block::Marching(2, 12345)];
    let mut indices = Box::new([0; 4096]);
    indices[0] = 1;
    chunk2.storage = BlockStorage::Sparse(indices);
    world.chunks.insert(ChunkCoord::new(1, 2, 3).into(), chunk2);
    
    let binary = world.to_binary();
    let restored = World::from_binary(&binary).unwrap();
    
    assert_eq!(world.chunks.len(), restored.chunks.len());
    for (coord, chunk) in &world.chunks {
        let restored_chunk = restored.chunks.get(coord).unwrap();
        assert_eq!(chunk.palette, restored_chunk.palette);
        assert_eq!(chunk.storage, restored_chunk.storage);
    }
}

#[test]
fn world_invalid_data() {
    // Empty data
    assert!(World::from_binary(&[]).is_none());
    
    // Missing chunk data
    assert!(World::from_binary(&[1, 0, 0, 0]).is_none()); // Says 1 chunk but no data
    
    // Invalid chunk data
    let mut data = vec![1, 0, 0, 0]; // 1 chunk
    data.extend_from_slice(&[0; 8]); // Coordinate
    data.push(1); // Chunk with 1 palette entry
    assert!(World::from_binary(&data).is_none()); // Missing palette entry
}

#[test]
fn save_load_single_chunk() {
    let mut world = World::empty();
    let coord = ChunkCoord::new(1, 2, 3);
    
    let mut chunk = Chunk::new();
    chunk.palette = vec![Block::Simple(42, BlockRotation::XplusYplus)];
    chunk.storage = BlockStorage::Uniform(0);
    world.chunks.insert(coord.into(), chunk);
    
    let chunk_data = world.save_chunk(coord).unwrap();
    let (restored_coord, restored_chunk) = World::load_chunk_binary(&chunk_data).unwrap();
    
    assert_eq!(coord, restored_coord);
    let original_chunk = world.chunks.get(&coord.into()).unwrap();
    assert_eq!(original_chunk.palette, restored_chunk.palette);
    assert_eq!(original_chunk.storage, restored_chunk.storage);
}

#[test]
fn save_load_nonexistent_chunk() {
    let world = World::empty();
    assert!(world.save_chunk(ChunkCoord::new(1, 2, 3)).is_none());
}

#[test]
fn save_load_entire_world() {
    let temp_dir = config::get_save_path().join("world_test");
    
    // Create a test world
    let world = create_dummy_world();

    game_state::start_world("some_test_world");
    
    // Set the game state
    config::get_gamestate().world_change(world.clone());

    {
	    let test_load = config::get_gamestate().world();
	    // Verify
	    assert_eq!(world.chunks.len(), test_load.chunks.len());
	    
	    // Check uniform chunk
	    let uniform_coord = ChunkCoord::new(0, 0, 0).into();
	    let original_uniform = world.chunks.get(&uniform_coord).unwrap();
	    let restored_uniform = test_load.chunks.get(&uniform_coord).unwrap();
	    assert_eq!(original_uniform.palette, restored_uniform.palette);
	    assert_eq!(original_uniform.storage, restored_uniform.storage);
	    
	    // Check sparse chunk
	    let sparse_coord = ChunkCoord::new(1, 2, 3).into();
	    let original_sparse = world.chunks.get(&sparse_coord).unwrap();
	    let restored_sparse = test_load.chunks.get(&sparse_coord).unwrap();
	    assert_eq!(original_sparse.palette, restored_sparse.palette);
	    assert_eq!(original_sparse.storage, restored_sparse.storage);
    }
    
    // Save and load
    save_entire_world(&temp_dir).unwrap();
    load_entire_world(&temp_dir).unwrap();

    {
        let restored = config::get_gamestate().world();
        
        // Verify
        assert_eq!(world.chunks.len(), restored.chunks.len());
        
        // Check uniform chunk
        let uniform_coord = ChunkCoord::new(0, 0, 0).into();
        let original_uniform = world.chunks.get(&uniform_coord).unwrap();
        let restored_uniform = restored.chunks.get(&uniform_coord).unwrap();
        assert_eq!(original_uniform.palette, restored_uniform.palette);
        assert_eq!(original_uniform.storage, restored_uniform.storage);
        
        // Check sparse chunk
        let sparse_coord = ChunkCoord::new(1, 2, 3).into();
        let original_sparse = world.chunks.get(&sparse_coord).unwrap();
        let restored_sparse = restored.chunks.get(&sparse_coord).unwrap();
        assert_eq!(original_sparse.palette, restored_sparse.palette);
        assert_eq!(original_sparse.storage, restored_sparse.storage);
    }
}

#[test]
fn world_serialization_roundtrip() {

    let world = create_dummy_world();
    
    // Serialize and deserialize
    let binary = world.to_binary();
    let restored = World::from_binary(&binary).unwrap();
    
    // Verify
    assert_eq!(world.chunks.len(), restored.chunks.len());
    
    // Check uniform chunk
    let uniform_coord = ChunkCoord::new(0, 0, 0).into();
    let original_uniform = world.chunks.get(&uniform_coord).unwrap();
    let restored_uniform = restored.chunks.get(&uniform_coord).unwrap();
    assert_eq!(original_uniform.palette, restored_uniform.palette);
    assert_eq!(original_uniform.storage, restored_uniform.storage);
    
    // Check sparse chunk
    let sparse_coord = ChunkCoord::new(1, 2, 3).into();
    let original_sparse = world.chunks.get(&sparse_coord).unwrap();
    let restored_sparse = restored.chunks.get(&sparse_coord).unwrap();
    assert_eq!(original_sparse.palette, restored_sparse.palette);
    assert_eq!(original_sparse.storage, restored_sparse.storage);
}

#[test]
fn load_invalid_world() {
    let temp_dir = config::get_save_path().join("world_test_invalid");
        
    // Try to load - should fail
    assert!(load_entire_world(&temp_dir).is_err());
    
    // Create invalid data file
    let world_dir = temp_dir.join("test");
    std::fs::create_dir_all(&world_dir).unwrap();
    std::fs::write(world_dir.join("data.dat"), "invalid data").unwrap();
    
    // Try to load - should fail
    assert!(load_entire_world(&temp_dir).is_err());
}

#[test]
fn world_serialization_to_disc() {

	let path = config::get_save_path().join("test");

    let world = create_dummy_world();

    {
	    let world_data = world.to_binary();

	    let world_dir = path.join("world");
	    fs::create_dir_all(&world_dir).expect(" ");  // Create directory first
	    
	    let file_path = world_dir.join("data.dat");
	    let temp_path = file_path.with_extension("tmp");
	    
	    // Write to temp file first
	    {
	        let mut file = File::create(&temp_path).expect(" ");
	        file.write_all(&world_data).expect(" ");
	    }
	    
	    // Atomic rename
	    fs::rename(temp_path, file_path).expect(" ");
    }


    let file_path = path.join("world").join("data.dat");
    
    let mut file = File::open(&file_path).expect(" ");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect(" ");

    println!("Loaded world data size: {} bytes", bytes.len());
    
    // Verify we have at least the chunk count (4 bytes)
    assert!(bytes.len() >= 4);
    
    let restored = World::from_binary(&bytes).expect(" ");


    // Verify
    assert_eq!(world.chunks.len(), restored.chunks.len());

    // Check uniform chunk
    let uniform_coord = ChunkCoord::new(0, 0, 0).into();
    let original_uniform = world.chunks.get(&uniform_coord).unwrap();
    let restored_uniform = restored.chunks.get(&uniform_coord).unwrap();
    assert_eq!(original_uniform.palette, restored_uniform.palette);
    assert_eq!(original_uniform.storage, restored_uniform.storage);
    
    // Check sparse chunk
    let sparse_coord = ChunkCoord::new(1, 2, 3).into();
    let original_sparse = world.chunks.get(&sparse_coord).unwrap();
    let restored_sparse = restored.chunks.get(&sparse_coord).unwrap();
    assert_eq!(original_sparse.palette, restored_sparse.palette);
    assert_eq!(original_sparse.storage, restored_sparse.storage);
        
}

#[cfg(test)]
pub fn create_dummy_world() -> World {

    let mut world = World::empty();
    
    // Add a uniform chunk
    let mut uniform_chunk = Chunk::new();
    uniform_chunk.palette = vec![Block::Simple(42, BlockRotation::XplusYplus)];
    uniform_chunk.storage = BlockStorage::Uniform(0);
    world.chunks.insert(ChunkCoord::new(0, 0, 0).into(), uniform_chunk);
    
    // Add a sparse chunk
    let mut sparse_chunk = Chunk::new();
    sparse_chunk.palette = vec![Block::None, Block::Marching(2, 12345)];
    let mut indices = Box::new([0; 4096]);
    indices[0] = 1;
    sparse_chunk.storage = BlockStorage::Sparse(indices);
    world.chunks.insert(ChunkCoord::new(1, 2, 3).into(), sparse_chunk);

    world
}