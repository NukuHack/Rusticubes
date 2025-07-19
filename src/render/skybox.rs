// Add to your existing code:

use crate::fs::rs;
use crate::State;

/// Struct to hold skybox resources
pub struct Skybox {
	pub texture: wgpu::Texture,
	pub bind_group: wgpu::BindGroup,
}

impl Skybox {
	/// Creates a new skybox from a texture path
	pub fn new(
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		layout: &wgpu::BindGroupLayout,
		skybox_path: &str,
	) -> Option<Self> {
		let texture = create_skybox_texture(device, queue, skybox_path)?;
		
		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&view),
				},
			],
			label: Some("skybox_bind_group"),
		});

		Some(Self { texture, bind_group })
	}
}


// Add a method to State to set skybox:
impl State<'_> {
	pub fn set_skybox(&mut self, skybox_path: &str) -> Result<(), String> {
		self.render_context.skybox = Skybox::new(
			self.device(),
			self.queue(),
			&self.render_context.skybox_bind_group_layout,
			skybox_path,
		).ok_or("Failed to create skybox")?;
		
		Ok(())
	}
}


/*{

	// Create a texture view
	let view = texture.create_view(&wgpu::TextureViewDescriptor {
		dimension: Some(wgpu::TextureViewDimension::D2),
		..Default::default()
	});

	// Create sampler
	let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wgpu::AddressMode::ClampToEdge,
		address_mode_v: wgpu::AddressMode::ClampToEdge,
		address_mode_w: wgpu::AddressMode::ClampToEdge,
		mag_filter: wgpu::FilterMode::Linear,
		min_filter: wgpu::FilterMode::Linear,
		mipmap_filter: wgpu::FilterMode::Linear,
		..Default::default()
	});


	let skybox_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: true },
					view_dimension: wgpu::TextureViewDimension::D2,
					multisampled: false,
				},
				count: None,
			},
		],
		label: Some("skybox_bind_group_layout"),
	});
	// Create bind group
	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &skybox_bind_group_layout,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: wgpu::BindingResource::Sampler(&sampler),
			},
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::TextureView(&view),
			},
		],
		label: Some("skybox_bind_group"),
	});
	let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("Render Pipeline Layout"),
		bind_group_layouts: &[
			// Your other bind group layouts...
			&skybox_bind_group_layout,
		],
		push_constant_ranges: &[],
	});

}*/

fn create_skybox_texture(device: &wgpu::Device, queue: &wgpu::Queue, skybox: &str) -> Option<wgpu::Texture> {
	// Load the image
	let (rgba, width, height) = rs::load_image_from_path(skybox)?;
	
	// Create the texture
	let texture = device.create_texture(&wgpu::TextureDescriptor {
		label: Some("skybox_texture"),
		size: wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1, // For a single skybox image
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: wgpu::TextureFormat::Rgba8Unorm,
		usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
		view_formats: &[],
	});

	// Write the image data to the texture
	queue.write_texture(
		wgpu::TexelCopyTextureInfo {
			texture: &texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
			aspect: wgpu::TextureAspect::All,
		},
		&rgba,
		wgpu::TexelCopyBufferLayout {
			offset: 0,
			bytes_per_row: Some(4 * width),
			rows_per_image: Some(height),
		},
		wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
	);

	Some(texture)
}