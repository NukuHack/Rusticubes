use std::borrow::Cow;
use wgpu::{Device, PipelineLayout, RenderPipeline, ShaderModule, ShaderModuleDescriptor, ShaderSource, SurfaceConfiguration};

use super::instances::*;
use super::{geometry, texture};

pub struct Pipeline {
    #[allow(unused)]
    pub render_pipeline: RenderPipeline,
    shader: ShaderModule, // Keep the shader alive as long as the pipeline exists
}

impl Pipeline {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        render_pipeline_layout: &PipelineLayout,
    ) -> Self {
        // Create the shader module and keep it in the struct to prevent dropping
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(Cow::from(include_str!("shader.wgsl"))),
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
                    geometry::TexCoord::desc(), // New buffer for texture coordinates
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