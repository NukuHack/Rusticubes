
use crate::physic::aabb::AABB;
use crate::render::meshing::{CUBE_FACES, InstanceRaw, ChunkMeshBuilder, GeometryBuffer};
use crate::block::math::{ChunkCoord, LocalPos};
use crate::block::main::{Block, Chunk, BlockStorage};
use crate::player::CameraSystem;
use crate::world::main::World;
use crate::ext::ptr;
use wgpu::util::DeviceExt;
use glam::IVec3;

pub struct NeighboringChunks<'a> {
	chunks: [Option<&'a Chunk>; 6],
}

impl<'a> NeighboringChunks<'a> {
	pub fn new(chunks: [Option<&'a Chunk>; 6]) -> Self {
		Self { chunks }
	}

	// Directional accessors
	pub fn left(&self) -> Option<&'a Chunk> { self.chunks[0] }    // (-1, 0, 0)
	pub fn right(&self) -> Option<&'a Chunk> { self.chunks[1] }    // (1, 0, 0)
	pub fn front(&self) -> Option<&'a Chunk> { self.chunks[2] }   // (0, 0, -1)
	pub fn back(&self) -> Option<&'a Chunk> { self.chunks[3] }    // (0, 0, 1)
	pub fn top(&self) -> Option<&'a Chunk> { self.chunks[4] }     // (0, 1, 0)
	pub fn bottom(&self) -> Option<&'a Chunk> { self.chunks[5] }  // (0, -1, 0)

	pub fn is_some(&self) -> bool {
		self.chunks.iter().all(Option::is_some)
	}

	// Add an iter() method that returns an iterator over Option<&Chunk>
	pub fn iter(&self) -> impl Iterator<Item = Option<&'a Chunk>> + '_ {
		self.chunks.iter().copied()
	}
}

// =============================================
// Extra Rendering related Implementations
// =============================================

impl Chunk {
	pub fn make_mesh(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, neighbors: NeighboringChunks) {
		if !self.dirty 
			&& (self.mesh().is_some() ^ self.is_empty()) 
			&& (self.final_mesh || !neighbors.is_some()) 
		{
			return;
		}

		// Early return if chunk is empty
		if self.is_empty() {
			self.set_mesh(Some(GeometryBuffer::empty(device)));
			self.dirty = false;
			self.final_mesh = neighbors.is_some();
			return;
		}

		let mut builder = ChunkMeshBuilder::new();

		// Optimize based on storage type
		match &self.storage() {
			BlockStorage::Uniform { block } => {
				self.make_mesh_uniform(*block, &mut builder, &neighbors);
			}
			BlockStorage::Compact { palette, indices } => {
				self.make_mesh_compact(&palette, &indices, &mut builder, &neighbors);
			}
			BlockStorage::Sparse { palette, indices } => {
				self.make_mesh_sparse(&palette, &indices, &mut builder, &neighbors);
			}
			BlockStorage::Rle { .. } => {
				// Never gonna happen ...
				todo!();
			}
			_ => {
				// Fallback to original method for Giant and Zigzag
				for pos_idx in 0..Self::VOLUME {
					let block = self.get_block(pos_idx);
					if block.is_empty() { continue; }

					self.add_cube_faces(pos_idx, block.material().inner(), &mut builder, &neighbors);
				}
			}
		}

		self.set_mesh(Some(builder.build(device)));
		self.dirty = false;
		self.final_mesh = neighbors.is_some();
	}

	#[inline]
	fn add_cube_faces(&self, pos: usize, material_id: u16, builder: &mut ChunkMeshBuilder, neighbors: &NeighboringChunks) {
		let local = LocalPos::from(pos);
		let pos = IVec3::from(local);
		let local_pos_packed = u16::from(local) as u32;
		
		// Unrolled loop for better performance
		// Face 0: Left (-X)
		if !self.should_cull_face(pos + IVec3::NEG_X, neighbors) {
			builder.instances.push(InstanceRaw {
				packed_data: local_pos_packed | (0u32 << 15) | (material_id as u32) << 19
			});
		}
		// 0-15 : pos ; 16-19 : rot ; 19 ... block id
		
		// Face 1: Right (+X)
		if !self.should_cull_face(pos + IVec3::X, neighbors) {
			builder.instances.push(InstanceRaw {
				packed_data: local_pos_packed | (1u32 << 15) | (material_id as u32) << 19
			});
		}
		
		// Face 2: Front (-Z)
		if !self.should_cull_face(pos + IVec3::NEG_Z, neighbors) {
			builder.instances.push(InstanceRaw {
				packed_data: local_pos_packed | (2u32 << 15) | (material_id as u32) << 19
			});
		}
		
		// Face 3: Back (+Z)
		if !self.should_cull_face(pos + IVec3::Z, neighbors) {
			builder.instances.push(InstanceRaw {
				packed_data: local_pos_packed | (3u32 << 15) | (material_id as u32) << 19
			});
		}
		
		// Face 4: Top (+Y)
		if !self.should_cull_face(pos + IVec3::Y, neighbors) {
			builder.instances.push(InstanceRaw {
				packed_data: local_pos_packed | (4u32 << 15) | (material_id as u32) << 19
			});
		}
		
		// Face 5: Bottom (-Y)
		if !self.should_cull_face(pos + IVec3::NEG_Y, neighbors) {
			builder.instances.push(InstanceRaw {
				packed_data: local_pos_packed | (5u32 << 15) | (material_id as u32) << 19
			});
		}
	}

	#[inline]
	fn make_mesh_uniform(&self, block: Block, builder: &mut ChunkMeshBuilder, neighbors: &NeighboringChunks) {
		let material_id = block.material().inner();
		
		// For uniform chunks, we can batch process faces more efficiently
		// Only generate faces on chunk boundaries and where neighbor chunks have different blocks
		
		// Process all positions, but with optimized boundary checks
		for z in 0..Self::SIZE {
			for y in 0..Self::SIZE {
				for x in 0..Self::SIZE {
					let pos = IVec3::new(x as i32, y as i32, z as i32);
					
					// Check each face direction
					for (face_idx, &normal) in CUBE_FACES.iter().enumerate() {
						let neighbor_pos = pos + normal;
						
						// Quick boundary checks for uniform blocks
						if self.should_cull_face_uniform(neighbor_pos, block, neighbors) { continue }

						let local_pos_packed = u16::from(LocalPos::from(pos)) as u32;
						builder.instances.push(InstanceRaw {
							packed_data: local_pos_packed | (face_idx as u32) << 15 | (material_id as u32) << 19
						});
					}
				}
			}
		}
	}

	#[inline]
	fn should_cull_face_uniform(&self, neighbor_pos: IVec3, _uniform_block: Block, neighbors: &NeighboringChunks) -> bool {
		// If neighbor is inside current chunk - cull it
		if self.contains_position(neighbor_pos) {
			return true; // Same block, cull the face
		}
		
		// Check neighboring chunk
		let Some(neighbor_chunk) = self.get_neighbor_chunk_from_pos(neighbor_pos, neighbors) else { return true; }; // No neighbor chunk - just cull for now

		let idx = usize::from(LocalPos::from(neighbor_pos));
		return !neighbor_chunk.get_block(idx).is_empty();
	}

	#[inline]
	fn make_mesh_compact(&self, palette: &[Block], indices: &Box<[u8; Self::VOLUME/2]>, builder: &mut ChunkMeshBuilder, neighbors: &NeighboringChunks) {
		// Pre-filter non-empty blocks from palette for faster lookup
		let non_empty_blocks: Vec<(usize, Block)> = palette.iter()
			.enumerate()
			.filter(|(_, block)| !block.is_empty())
			.map(|(i, &block)| (i, block))
			.collect();
		
		if non_empty_blocks.is_empty() {
			return; // All blocks are empty
		}

		// Directly work with the array without extra dereferencing
		let indices = &**indices; // Deref once to &[u8]
	
		// Process positions in a tight loop
		for pos_idx in 0..Self::VOLUME {
			let byte_idx = pos_idx / 2;
			let palette_idx = if pos_idx % 2 == 1 {
				(indices[byte_idx] >> 4) & 0x0F
			} else {
				indices[byte_idx] & 0x0F
			} as usize;

			// Skip if this palette entry is empty or out of bounds
			if palette_idx >= palette.len() || palette[palette_idx].is_empty() {
				continue;
			}

			let block = palette[palette_idx];

			// Generate faces for this block
			self.add_cube_faces(pos_idx, block.material().inner(), builder, neighbors);
		}
	}

	#[inline]
	fn make_mesh_sparse(&self, palette: &[Block], indices: &Box<[u8; Self::VOLUME]>, builder: &mut ChunkMeshBuilder, neighbors: &NeighboringChunks) {
		// Similar to compact but with direct index access
		for pos_idx in 0..Self::VOLUME {
			let palette_idx = indices[pos_idx] as usize;
			
			if palette_idx >= palette.len() || palette[palette_idx].is_empty() {
				continue;
			}
			
			let block = palette[palette_idx];
			
			self.add_cube_faces(pos_idx, block.material().inner(), builder, neighbors);
		}
	}

	#[inline]
	fn should_cull_face(&self, neighbor_pos: IVec3, neighbors: &NeighboringChunks) -> bool {
		// Check if position is inside current chunk
		let idx = usize::from(LocalPos::from(neighbor_pos));
		if self.contains_position(neighbor_pos) {
			return !self.get_block(idx).is_empty();
		}
		
		// Check neighboring chunk
		let Some(neighbor_chunk) = self.get_neighbor_chunk_from_pos(neighbor_pos, neighbors) else { return true; }; // No neighbor chunk - just cull for now

		return !neighbor_chunk.get_block(idx).is_empty();
	}

	#[inline]
	fn get_neighbor_chunk_from_pos<'a>(&self, neighbor_pos: IVec3, neighbors: &NeighboringChunks<'a>) -> Option<&'a Chunk> {
		const LEFT_EDGE: i32 = -1;
		const RIGHT_EDGE: i32 = Chunk::SIZE_I;
		
		// Optimized neighbor lookup
		match (neighbor_pos.x, neighbor_pos.y, neighbor_pos.z) {
			(LEFT_EDGE, _, _) => neighbors.left(),
			(RIGHT_EDGE, _, _) => neighbors.right(),
			(_, _, LEFT_EDGE) => neighbors.front(),
			(_, _, RIGHT_EDGE) => neighbors.back(),
			(_, RIGHT_EDGE, _) => neighbors.top(),
			(_, LEFT_EDGE, _) => neighbors.bottom(),
			_ => None,
		}
	}

	/// Recreates chunk's bind group
	pub fn create_bind_group(&mut self, chunk_coord: ChunkCoord) {
		if self.bind_group().is_some() { return; }
		let state = ptr::get_state();
		let device = state.device();
		let chunk_bind_group_layout = &state.render_context.layouts[2];

		// Create position buffer
		let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Chunk Position Buffer"),
			contents: bytemuck::cast_slice(&[u64::from(chunk_coord)]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		// Create bind group
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: chunk_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: position_buffer.as_entire_binding(),
			}],
			label: Some("chunk_bind_group"),
		});

		self.set_bind_group(Some(bind_group));
	}
}
impl World {
	/// Generates meshes for all dirty chunks and all non final meshed ones
	#[inline]
	pub fn make_chunk_meshes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
		
		// Get raw pointer to the world's chunks
		let world_ptr = self as *mut World;

		for (chunk_coord, chunk) in self.chunks.iter_mut() {
			if chunk.is_empty() { continue; }

			// SAFETY:
			// 1. We only use the pointer to access different chunks than the one we're modifying
			// 2. The references don't outlive this scope
			// 3. We don't modify through these references
			let neighbors = unsafe {
				let world_ref = &*world_ptr;
				world_ref.get_neighboring_chunks(*chunk_coord)
			};

			chunk.make_mesh(device, queue, neighbors);
		}
	}

	#[inline]
	pub fn get_neighboring_chunks(&self, chunk_coord: ChunkCoord) -> NeighboringChunks {
		//self.chunks.get(&chunk_coord);
		let neigh = chunk_coord.get_adjacent();
		NeighboringChunks::new([
			self.get_chunk(neigh[0]),  // Left
			self.get_chunk(neigh[1]), // Right
			self.get_chunk(neigh[2]),   // Front
			self.get_chunk(neigh[3]), // Back
			self.get_chunk(neigh[4]), // Top
			self.get_chunk(neigh[5]),   // Bottom
		])
	}

	// Improved rendering function with better culling
	pub fn render_chunks_with_culling<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, cam_sys: &CameraSystem, max_render_distance: f32) {
		let frustum = cam_sys.frustum();
		let camera_pos = cam_sys.uniform().to_pos_vec3();
		
		// Pre-calculate max_render_distance_squared for faster comparisons
		let max_render_distance_squared = max_render_distance * max_render_distance;
		
		for (chunk_coord, chunk) in self.chunks.iter() {
			if chunk.is_empty() { continue; }
			let (Some(mesh), Some(bind_group)) = (&chunk.mesh(), &chunk.bind_group()) else { continue };
			if mesh.num_instances == 0 { continue }
			
			let chunk_aabb = AABB::from_chunk_coord(&chunk_coord);
			
			// Distance culling first (cheaper) - using squared distance to avoid sqrt
			let chunk_center = chunk_aabb.center();
			let distance_squared_to_camera = (chunk_center - camera_pos).length_squared();
			if distance_squared_to_camera > max_render_distance_squared { continue }
			
			// Enhanced frustum culling with detailed intersection info
			if !frustum.contains_aabb(&chunk_aabb) { continue }

			// Inside or intersecting frustum - render it
			render_pass.set_bind_group(2, *bind_group, &[]);
			render_pass.set_vertex_buffer(1, mesh.instance_buffer.slice(..));
			render_pass.draw(0..6, 0..mesh.num_instances as u32);
		}
	}

	pub fn create_bind_group(&mut self, chunk_coord: ChunkCoord) {
		if !self.loaded_chunks.contains(&chunk_coord) { return; }
		let Some(c) = self.get_chunk_mut(chunk_coord) else { return; };
		
		c.create_bind_group(chunk_coord);
	}
}
