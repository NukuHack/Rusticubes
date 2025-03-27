use std::borrow::Cow;
use wgpu::{Device, PipelineLayout, RenderPipeline, ShaderModule, ShaderModuleDescriptor, ShaderSource, SurfaceConfiguration};

use super::instances::*;
use super::{geometry, texture};

#[allow(dead_code,unused,redundant_imports,unused_results,unused_features,unused_variables,unused_mut,dead_code,unused_unsafe,unused_attributes)]
pub struct Pipeline {
    pub render_pipeline: RenderPipeline,
    shader: ShaderModule, // Keep the shader alive as long as the pipeline exists
}

impl Pipeline {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        render_pipeline_layout: &PipelineLayout,
    ) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(Cow::from(include_str!("texture_shader.wgsl"))),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Option::from("vs_main"),

                compilation_options: Default::default(),
                buffers: &[
                    geometry::Vertex::desc(),
                    InstanceRaw::desc(), // Instance data buffer
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Option::from("fs_main"),

                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw, // Ensure this matches winding
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(), // Explicitly set to default for clarity
            multiview: None,
            cache: None,
        });

        Pipeline { render_pipeline, shader }
    }
    pub fn new_old(
        device: &Device,
        config: &SurfaceConfiguration,
        render_pipeline_layout: &PipelineLayout,
    ) -> Self {
        // Create the shader module and keep it in the struct to prevent dropping
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(Cow::from(
                include_str!(r#"texture_shader.wgsl"#))),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Option::from("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    geometry::Vertex::desc(),
                    //geometry::TexCoord::desc(), // New buffer for texture coordinates
                    InstanceRaw::desc(),
                ],
                // compilation_options are default, so they can be omitted
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Option::from("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                // compilation_options are default, so they can be omitted
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
            multisample: wgpu::MultisampleState::default(), // Explicitly set to default for clarity
            multiview: None,
            cache: None,
        });

        Pipeline {
            render_pipeline,
            shader, // Ensure shader is stored to keep it alive
        }
    }
}

const BACKGROUND_COLOR: wgpu::Color = wgpu::Color
{    r: 0.1,    g: 0.2,    b: 0.3,    a: 1.0, };

fn begin_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    })
}
fn draw_geometry(
    pass: &mut wgpu::RenderPass,
    pipeline: &wgpu::RenderPipeline,
    bind_groups: &[&wgpu::BindGroup], // Use a slice of bind groups
    vertex_buffers: &[&wgpu::Buffer],
    index_buffer: &wgpu::Buffer,
    num_indices: u32,
    instances: usize,
) {
    pass.set_pipeline(pipeline);
    for (i, bind_group) in bind_groups.iter().enumerate() {
        pass.set_bind_group(i as _, *bind_group, &[]); // Dereference the bind group
    }
    for (i, buffer) in vertex_buffers.iter().enumerate() {
        pass.set_vertex_buffer(i as _, buffer.slice(..));
    }
    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..num_indices, 0, 0..instances as _);
}



pub fn render_all(current_state: &super::State) -> Result<(), wgpu::SurfaceError> {
    let output = current_state.surface.get_current_texture()?;
    let view = output.texture.create_view(&Default::default());
    let depth_view = &current_state.texture_manager.depth_texture.view;
    let mut encoder = current_state.device.create_command_encoder(&Default::default());
    {
        let mut pass = begin_render_pass(&mut encoder, &view, depth_view);

        draw_geometry(
            &mut pass,
            &current_state.pipeline.render_pipeline,
            &[&current_state.texture_manager.bind_group, &current_state.camera_system.bind_group], // Pass bind groups as a slice
            &[
                &current_state.geometry_buffer.vertex_buffer,
                &current_state.geometry_buffer.texture_coord_buffer,
                &current_state.instance_manager.instance_buffer,
            ],
            &current_state.geometry_buffer.index_buffer,
            current_state.geometry_buffer.num_indices,
            current_state.instance_manager.instances.len(),
        );
    }
    current_state.queue.submit(Some(encoder.finish()));
    output.present();

    Ok(())
}