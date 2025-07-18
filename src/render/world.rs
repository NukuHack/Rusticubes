
use wgpu::util::DeviceExt;
use ahash::AHasher;
use glam::IVec3;
use std::{ collections::HashMap, hash::BuildHasherDefault };
use crate::block::main::{Block, Chunk};
use crate::block::math::{ChunkCoord, BlockPosition};
use crate::render::meshing::{ChunkMeshBuilder, GeometryBuffer};
use crate::ext::ptr;
use crate::world::main::World;

// =============================================
// Extra Rendering related Implementations
// =============================================
/*
#[derive(Clone, PartialEq)]
pub struct Chunk {
	pub palette: Vec<Block>, 
	pub storage: BlockStorage, 
	pub dirty: bool, 
	pub mesh: Option<GeometryBuffer>, 
	pub bind_group: Option<wgpu::BindGroup>, 
}
*/
impl Chunk {
	pub fn make_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, neighbors: &[Option<&Chunk>; 6], force: bool) {
		if !force && !self.dirty && self.mesh.is_some() {
			return;
		}

		// Early return if chunk is empty
		if self.is_empty() {
			if self.mesh.is_some() {
				self.mesh = Some(GeometryBuffer::empty(device));
				self.dirty = false;
			}
			return;
		}

		let mut builder = ChunkMeshBuilder::new();

		for pos in 0..Self::VOLUME {
			let block = *self.get_block(pos);
			if block.is_empty() {
				continue;
			}
			let local_pos:IVec3 = BlockPosition::from(pos).into();
			match block {
				Block::Simple(..) => {
					builder.add_cube(local_pos, block.texture_coords(), self, neighbors);
				},
				_ => {},
			}
		}

		if let Some(mesh) = &mut self.mesh {
			mesh.update(device, queue, &builder.indices, &builder.vertices);
		} else {
			self.mesh = Some(GeometryBuffer::new(
				device,
				&builder.indices,
				&builder.vertices,
			));
		}
		self.dirty = false;
	}
}
impl World {
	/// Generates meshes for all dirty chunks
	#[inline]
	pub fn make_chunk_meshes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
		// Define the full HashMap type including the hasher
		type ChunkMap = HashMap<ChunkCoord, Chunk, BuildHasherDefault<AHasher>>;
		
		// Get raw pointer with explicit type
		let chunks_ptr: *const ChunkMap = &self.chunks as *const ChunkMap;
		
		for (chunk_pos, chunk) in self.chunks.iter_mut() {
			if chunk.is_empty() {
				continue;
			}

			// SAFETY:
			// 1. We only access different chunks than the one we're mutating
			// 2. The references don't outlive the current iteration
			// 3. We don't modify the chunks through these references
			let neighbors = unsafe {
				let chunks: &ChunkMap = &*chunks_ptr;
				[
					chunks.get(&chunk_pos.offset(-1, 0, 0)),  // Left
					chunks.get(&chunk_pos.offset(1, 0, 0)),   // Right
					chunks.get(&chunk_pos.offset(0, 0, -1)),  // Front
					chunks.get(&chunk_pos.offset(0, 0, 1)),   // Back
					chunks.get(&chunk_pos.offset(0, 1, 0)),   // Top
					chunks.get(&chunk_pos.offset(0, -1, 0)),  // Bottom
				]
			};

			chunk.make_mesh(device, queue, &neighbors, false);
		}
	}

	#[inline]
	pub fn get_neighboring_chunks(&self, chunk_pos: ChunkCoord) -> [Option<&Chunk>;6] {
		self.chunks.get(&chunk_pos);

		[
			self.chunks.get(&chunk_pos.offset(-1, 0, 0)),  // Left
			self.chunks.get(&chunk_pos.offset(1, 0, 0)),   // Right
			self.chunks.get(&chunk_pos.offset(0, 0, -1)),  // Front
			self.chunks.get(&chunk_pos.offset(0, 0, 1)),   // Back
			self.chunks.get(&chunk_pos.offset(0, 1, 0)),    // Top
			self.chunks.get(&chunk_pos.offset(0, -1, 0)),   // Bottom
		]
	}
}
impl Chunk {
	/// Recreates chunk's bind group
	pub fn create_bind_group(&mut self, chunk_pos: ChunkCoord) {
		let state = ptr::get_state();
		let device = state.device();
		let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

		// Create position buffer
		let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Chunk Position Buffer"),
			contents: bytemuck::cast_slice(&[
				chunk_pos.into(),
				0.0 as u64,
			]),
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
		for (_chunk_pos, chunk) in self.chunks.iter() {
			// Skip empty chunks entirely - no mesh or bind group needed
			if chunk.is_empty() {
				continue;
			}
			if let (Some(mesh), Some(bind_group)) = (&chunk.mesh, &chunk.bind_group) {
				// Skip if mesh has no indices (shouldn't happen but good to check)
				if mesh.num_indices == 0 {
					continue;
				}

				render_pass.set_bind_group(2, bind_group, &[]);
				render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
				render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
				render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
			}
		}
	}

	pub fn remake_rendering(&mut self) {
		for (coord, chunk) in self.chunks.iter_mut() {
			chunk.create_bind_group(*coord);
			if !self.loaded_chunks.contains(&coord) {
				self.loaded_chunks.insert(*coord);
			}
		}
	}

	pub fn create_bind_group(&mut self, chunk_coord: ChunkCoord) {
		if self.loaded_chunks.contains(&chunk_coord) {
			match self.get_chunk_mut(chunk_coord) {
				Some(c) => {
					c.create_bind_group(chunk_coord);
				},
				None => {}
			}
		}
	}
}