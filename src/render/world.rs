
use crate::block::math::ChunkCoord;
use crate::config;
use crate::world::main::World;
use crate::block::main::Chunk;
use wgpu::util::DeviceExt;

// =============================================
// Extra Rendering related Implementations
// =============================================

impl Chunk {
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