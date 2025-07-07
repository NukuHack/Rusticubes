
use crate::block::main::Block;
use crate::block::math::{ChunkCoord, BlockPosition};
use crate::render::meshing::{ChunkMeshBuilder, GeometryBuffer};
use crate::config;
use crate::world::main::World;
use crate::block::main::Chunk;
use wgpu::util::DeviceExt;

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
    pub fn make_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, force: bool) {
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

            let local_pos = BlockPosition::from(pos).into();
            match block {
                Block::Marching(_, points) => {
                    builder.add_marching_cube(points, local_pos);
                }
                _ => {
                    builder.add_cube(local_pos, block.texture_coords(), self);
                }
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

    /// Recreates chunk's bind group
    pub fn recreate_bind_group(&mut self, chunk_coord: ChunkCoord) {
        let state = config::get_state();
        let device = state.device();
        let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

        // Create position buffer
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Position Buffer"),
            contents: bytemuck::cast_slice(&[
                chunk_coord.into(),
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
        self.make_mesh(device, state.queue(), true);
    }
}

impl World {
    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for chunk in self.chunks.values() {
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
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }

    pub fn remake_rendering(&mut self) {
        for (coord, chunk) in self.chunks.iter_mut() {
            chunk.recreate_bind_group(*coord);
            if !self.loaded_chunks.contains(&coord) {
                self.loaded_chunks.insert(*coord);
            }
        }

    }
}