
use std::borrow::Cow;
use wgpu;

use super::instances::*;
use super::{geometry, texture};

#[allow(dead_code, unused, redundant_imports, unused_results, unused_features, unused_variables, unused_mut, dead_code, unused_unsafe, unused_attributes)]
pub struct Pipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    shader: wgpu::ShaderModule, // Keep the shader alive as long as the pipeline exists
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        render_pipeline_layout: &wgpu::PipelineLayout,
    ) -> Self {
        // Create the shader module and keep it in the struct to prevent dropping
        let shader: wgpu::ShaderModule = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(include_str!("texture_shader.wgsl"))),
        });

        let render_pipeline: wgpu::RenderPipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    geometry::Vertex::desc(),
                    geometry::TexCoord::desc(),
                    InstanceRaw::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Pipeline {
            render_pipeline,
            shader, // Ensure shader is stored to keep it alive
        }
    }
}

const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,    g: 0.2,    b: 0.3,    a: 1.0,
};

// begin_render_pass now uses let bindings for clarity and breaks down components
fn begin_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    let color_attachment = wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
            store: wgpu::StoreOp::Store,
        },
    };

    let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: depth_view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    };

    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(color_attachment)],
        depth_stencil_attachment: Some(depth_attachment),
        ..Default::default()
    })
}

// draw_geometry now uses fixed-size arrays for bind_groups and vertex_buffers
pub fn draw_geometry(
    rpass: &mut wgpu::RenderPass,
    pipeline: &wgpu::RenderPipeline,
    bind_groups: Box<[&wgpu::BindGroup]>,
    vertex_buffers: Box<[&wgpu::Buffer]>,
    index_buffer: &wgpu::Buffer,
    num_indices: u32,
    instances: u32,
) {
    rpass.set_pipeline(pipeline);

    // Set bind groups with explicit slot indices
    for (i, bind_group) in bind_groups.iter().enumerate() {
        rpass.set_bind_group(i as u32, *bind_group, &[]);
    }

    // Set vertex buffers with explicit slot indices
    for (i, buffer) in vertex_buffers.iter().enumerate() {
        rpass.set_vertex_buffer(i as u32, buffer.slice(..));
    }

    // Set index buffer and draw with proper instance count
    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    rpass.draw_indexed(0..num_indices, 0, 0..instances);
}

pub fn render_all(current_state: &super::State) -> Result<(), wgpu::SurfaceError> {
    let output = current_state.surface.get_current_texture()?;
    let view = output.texture.create_view(&Default::default());
    let depth_view = &current_state.texture_manager.depth_texture.view;
    let mut encoder = current_state.device.create_command_encoder(&Default::default());

    {
        let mut rpass = begin_render_pass(&mut encoder, &view, depth_view);

        // Draw call with type-safe parameters
        draw_geometry(
            &mut rpass,
            &current_state.pipeline.render_pipeline,
            Box::from([
                &current_state.texture_manager.bind_group,
                &current_state.camera_system.bind_group,
            ]),
            Box::from([
                &current_state.geometry_buffer.vertex_buffer,
                &current_state.geometry_buffer.texture_coord_buffer,
                &current_state.instance_manager.instance_buffer,
            ]),
            &current_state.geometry_buffer.index_buffer,
            current_state.geometry_buffer.num_indices,
            current_state.instance_manager.instances.len() as u32,
        );
    }
    current_state.queue.submit(Some(encoder.finish()));
    output.present();

    Ok(())
}
