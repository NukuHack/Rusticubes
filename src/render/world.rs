
use wgpu::util::DeviceExt;
use glam::IVec3;
use crate::block::main::Chunk;
use crate::block::math::{ChunkCoord, BlockPosition};
use crate::render::meshing::{ChunkMeshBuilder, GeometryBuffer};
use crate::ext::ptr;
use crate::world::main::World;

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
			&& (self.mesh.is_some() ^ self.is_empty()) 
			&& (self.final_mesh || !neighbors.is_some()) 
		{
			return;
		}

		// Early return if chunk is empty
		if self.is_empty() && self.mesh.is_some() {
			self.mesh = Some(GeometryBuffer::empty(device));
			self.dirty = false;
			self.final_mesh = true;
		}

		let mut builder = ChunkMeshBuilder::new();

		for pos in 0..Self::VOLUME {
			let block = self.get_block(pos);
			if block.is_empty() { continue; }
			let local_pos:IVec3 = BlockPosition::from(pos).into();
			builder.add_cube(local_pos, block.material().inner(), &self, &neighbors);
		}

		self.mesh = Some(builder.build(device));
		self.dirty = false;
		if neighbors.is_some() {
			self.final_mesh = true;
		} else {
			self.final_mesh = false;
		}
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
}
impl Chunk {
	/// Recreates chunk's bind group
	pub fn create_bind_group(&mut self, chunk_coord: ChunkCoord) {
		if self.bind_group.is_some() { return; }
		let state = ptr::get_state();
		let device = state.device();
		let chunk_bind_group_layout = &state.render_context.layouts[2];

		// Create position buffer
		let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Chunk Position Buffer"),
			contents: bytemuck::cast_slice(&[<ChunkCoord as Into<u64>>::into(chunk_coord)]),
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

		self.bind_group = Some(bind_group);
	}
}


impl World {
	pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
		for (_chunk_coord, chunk) in self.chunks.iter() {
			// Skip empty chunks entirely - no mesh or bind group needed
			if chunk.is_empty() { continue; }
			let (Some(mesh), Some(bind_group)) = (&chunk.mesh, &chunk.bind_group) else { continue; };

			// Skip if mesh has no indices (shouldn't happen but good to check)
			if mesh.num_instances == 0 { continue; }

			render_pass.set_bind_group(2, bind_group, &[]);
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
