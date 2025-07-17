
use crate::State;
use crate::ext::config;
use crate::render::meshing::Vertex;
use crate::render::texture;
use crate::get_string;
use wgpu;

/// Struct holding all render pipelines and their associated shaders
#[allow(dead_code)]
pub struct Pipeline {
	// Pipelines
	inside_pipeline: wgpu::RenderPipeline,
	chunk_pipeline: wgpu::RenderPipeline,
	post_pipeline: wgpu::RenderPipeline,
	sky_pipeline: wgpu::RenderPipeline,
	// Cached layouts
	post_bind_group_layout: wgpu::BindGroupLayout,
	post_pipeline_layout: wgpu::PipelineLayout,
}

impl Pipeline {
	/// Creates all render pipelines with proper configuration
	#[inline]
	pub fn new(
		device: &wgpu::Device,
		config: &wgpu::SurfaceConfiguration,
		layout: &wgpu::PipelineLayout,
	) -> Self {
		// Create shaders
		let shaders = Shaders::new(device);

		// Create post processing bind group layout and pipeline layout once
		let post_bind_group_layout = create_post_bind_group_layout(device);
		let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Post Processing Pipeline Layout"),
			bind_group_layouts: &[&post_bind_group_layout],
			push_constant_ranges: &[],
		});

		Self {
			inside_pipeline: create_inside_pipeline(device, layout, &shaders.inside, config.format),
			chunk_pipeline: create_chunk_pipeline(device, layout, &shaders.chunk, config.format),
			post_pipeline: create_post_pipeline(
				device,
				&shaders.post,
				config.format,
				&post_pipeline_layout,
			),
			sky_pipeline: create_sky_pipeline(device, &shaders.sky, config.format),

			// Store layouts for reuse
			post_bind_group_layout,
			post_pipeline_layout,
		}
	}
	#[inline]
	pub fn post_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
		&self.post_bind_group_layout
	}
	#[inline]
	pub fn inside_pipeline(&self) -> &wgpu::RenderPipeline {
		&self.inside_pipeline
	}
	#[inline]
	pub fn chunk_pipeline(&self) -> &wgpu::RenderPipeline {
		&self.chunk_pipeline
	}
	#[inline]
	pub fn post_pipeline(&self) -> &wgpu::RenderPipeline {
		&self.post_pipeline
	}
	#[inline]
	pub fn sky_pipeline(&self) -> &wgpu::RenderPipeline {
		&self.sky_pipeline
	}
}

/// Helper struct for organizing shader creation
struct Shaders {
	inside: wgpu::ShaderModule,
	chunk: wgpu::ShaderModule,
	post: wgpu::ShaderModule,
	sky: wgpu::ShaderModule,
}

impl Shaders {
	#[inline]
	fn new(device: &wgpu::Device) -> Self {
		// Load shader sources first
		let chunk_shader = get_string!("chunk_shader.wgsl");
		let inside_shader = get_string!("inside_shader.wgsl");
		let texture_shader = get_string!("texture_shader.wgsl");
		let sky_shader = get_string!("sky_shader.wgsl");
		let fxaa_shader = get_string!("fxaa.wgsl");

		// Combine shader sources using string concatenation
		let inside_source = format!("{}\n{}", chunk_shader, inside_shader);
		let chunk_source = format!("{}\n{}", chunk_shader, texture_shader);

		Self {
			inside: create_shader(device, "Inside Solid Color Shader", &inside_source),
			chunk: create_shader(device, "Chunk Shader", &chunk_source),
			post: create_shader(device, "Post Processing Shader", &fxaa_shader),
			sky: create_shader(device, "Sky Shader", &sky_shader),
		}
	}
}

/// Creates a shader module with the given label and source
#[inline]
fn create_shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
	device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: Some(label),
		source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(source)),
	})
}

/// Common pipeline creation helper
#[inline]
fn create_base_pipeline(
	device: &wgpu::Device,
	layout: Option<&wgpu::PipelineLayout>,
	shader: &wgpu::ShaderModule,
	format: wgpu::TextureFormat,
	buffers: &[wgpu::VertexBufferLayout],
	depth_stencil: Option<wgpu::DepthStencilState>,
	primitive: wgpu::PrimitiveState,
	label: &str,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some(label),
		layout,
		vertex: wgpu::VertexState {
			module: shader,
			entry_point: Some("vs_main"),
			compilation_options: Default::default(),
			buffers,
		},
		fragment: Some(wgpu::FragmentState {
			module: shader,
			entry_point: Some("fs_main"),
			compilation_options: Default::default(),
			targets: &[Some(wgpu::ColorTargetState {
				format,
				blend: Some(wgpu::BlendState::ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})],
		}),
		primitive,
		depth_stencil,
		multisample: wgpu::MultisampleState::default(),
		multiview: None,
		cache: None,
	})
}
#[inline]
fn create_chunk_pipeline(
	device: &wgpu::Device,
	layout: &wgpu::PipelineLayout,
	shader: &wgpu::ShaderModule,
	format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
	create_base_pipeline(
		device,
		Some(layout),
		shader,
		format,
		&[Vertex::desc()],
		Some(depth_stencil_state(true)),
		default_primitive_state(),
		"Chunk Render Pipeline",
	)
}
#[inline]
fn create_inside_pipeline(
	device: &wgpu::Device,
	layout: &wgpu::PipelineLayout,
	shader: &wgpu::ShaderModule,
	format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
	create_base_pipeline(
		device,
		Some(layout),
		shader,
		format,
		&[Vertex::desc()],
		Some(inside_depth_stencil()),
		inside_primitive_state(),
		"Inside Render Pipeline",
	)
}
#[inline]
fn create_post_pipeline(
	device: &wgpu::Device,
	shader: &wgpu::ShaderModule,
	format: wgpu::TextureFormat,
	layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
	create_base_pipeline(
		device,
		Some(layout),
		shader,
		format,
		&[],
		None,
		default_primitive_state(),
		"Post Processing Pipeline",
	)
}
#[inline]
fn create_post_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("Post Processing Bind Group Layout"),
		entries: &[
			// Texture
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: true },
					view_dimension: wgpu::TextureViewDimension::D2,
					multisampled: false,
				},
				count: None,
			},
			// Sampler
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			},
		],
	})
}
#[inline]
fn create_sky_pipeline(
	device: &wgpu::Device,
	shader: &wgpu::ShaderModule,
	format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Sky Render Pipeline"),
		layout: None,
		vertex: wgpu::VertexState {
			module: shader,
			entry_point: Some("vs_main"),
			compilation_options: Default::default(),
			buffers: &[],
		},
		fragment: Some(wgpu::FragmentState {
			module: shader,
			entry_point: Some("fs_main"),
			compilation_options: Default::default(),
			targets: &[Some(wgpu::ColorTargetState {
				format,
				blend: None,
				write_mask: wgpu::ColorWrites::ALL,
			})],
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			..Default::default()
		},
		depth_stencil: Some(wgpu::DepthStencilState {
			format: texture::DEPTH_FORMAT,
			depth_write_enabled: false,
			depth_compare: wgpu::CompareFunction::LessEqual,
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState::default(),
		multiview: None,
		cache: None,
	})
}

// --- Pipeline Configuration ---
#[inline]
fn default_primitive_state() -> wgpu::PrimitiveState {
	wgpu::PrimitiveState {
		topology: wgpu::PrimitiveTopology::TriangleList,
		strip_index_format: None,
		front_face: wgpu::FrontFace::Ccw,
		cull_mode: Some(wgpu::Face::Back),
		polygon_mode: wgpu::PolygonMode::Fill,
		conservative: false,
		unclipped_depth: false,
		..Default::default()
	}
}
#[inline]
fn inside_primitive_state() -> wgpu::PrimitiveState {
	wgpu::PrimitiveState {
		front_face: wgpu::FrontFace::Cw,
		..default_primitive_state()
	}
}
#[inline]
fn depth_stencil_state(write_enabled: bool) -> wgpu::DepthStencilState {
	wgpu::DepthStencilState {
		format: texture::DEPTH_FORMAT,
		depth_write_enabled: write_enabled,
		depth_compare: wgpu::CompareFunction::Less,
		stencil: wgpu::StencilState::default(),
		bias: wgpu::DepthBiasState::default(),
	}
}
#[inline]
fn inside_depth_stencil() -> wgpu::DepthStencilState {
	wgpu::DepthStencilState {
		depth_compare: wgpu::CompareFunction::Less,
		..depth_stencil_state(false)
	}
}

// --- Optimized Render Passes ---
#[inline]
pub fn render_all(current_state: &mut State) -> Result<(), wgpu::SurfaceError> {
	let output = current_state.surface().get_current_texture()?;
	let view = output
		.texture
		.create_view(&wgpu::TextureViewDescriptor::default());
	let mut encoder =
		current_state
			.device()
			.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: Some("Render Encoder"),
			});

	// Reusable render pass descriptors
	let binding = current_state.texture_manager().depth_texture().create_view(&wgpu::TextureViewDescriptor::default());
	let sky_pass_descriptor = wgpu::RenderPassDescriptor {
		label: Some("Sky Render Pass"),
		color_attachments: &[Some(wgpu::RenderPassColorAttachment {
			view: &view,
			resolve_target: None,
			ops: wgpu::Operations {
				load: wgpu::LoadOp::Load,
				store: wgpu::StoreOp::Store,
			},
		})],
		depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
			view: &binding,
			depth_ops: Some(wgpu::Operations {
				load: wgpu::LoadOp::Clear(1.0),
				store: wgpu::StoreOp::Store,
			}),
			stencil_ops: None,
		}),
		occlusion_query_set: None,
		timestamp_writes: None,
	};

	let mut sky_pass = encoder.begin_render_pass(&sky_pass_descriptor);
	sky_pass.set_pipeline(&current_state.pipeline.sky_pipeline);
	sky_pass.draw(0..3, 0..1);
	drop(sky_pass);

	// 3D pass
	if current_state.is_world_running {
		let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("3D Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
				view: &binding,
				depth_ops: Some(wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				}),
				stencil_ops: None,
			}),
			occlusion_query_set: None,
			timestamp_writes: None,
		});

		// Render chunks
		rpass.set_pipeline(&current_state.pipeline.chunk_pipeline);
		rpass.set_bind_group(0, current_state.texture_manager().bind_group(), &[]);
		rpass.set_bind_group(1, current_state.camera_system().bind_group(), &[]);

		config::get_gamestate()
			.world()
			.render_chunks(&mut rpass);

		// Render inside surfaces
		rpass.set_pipeline(&current_state.pipeline().inside_pipeline);
		rpass.set_bind_group(0, current_state.texture_manager().bind_group(), &[]);
		rpass.set_bind_group(1, current_state.camera_system().bind_group(), &[]);
		config::get_gamestate()
			.world()
			.render_chunks(&mut rpass);
	}

	// Post processing pass 
	{
		let mut post_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("FXAA Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			occlusion_query_set: None,
			timestamp_writes: None,
		});

		post_pass.set_pipeline(&current_state.pipeline.post_pipeline);
		post_pass.set_bind_group(0,current_state.texture_manager().post_processing_bind_group(),&[],);
		post_pass.draw(0..3, 0..1);
	}

	// UI pass 
	{
		let mut ui_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("UI Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			occlusion_query_set: None,
			timestamp_writes: None,
		});

		// Use the UIManager's render method instead of manual rendering
		current_state.ui_manager.render(&mut ui_rpass);
	}

	// Submit commands
	current_state
		.queue()
		.submit(std::iter::once(encoder.finish()));
	output.present();

	Ok(())
}
