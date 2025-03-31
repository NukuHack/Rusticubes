#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    uv: [f32; 2],
}

#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
pub struct UIElement {
    pub position: (f32, f32), // Normalized coordinates (-1.0 to 1.0)
    pub size: (f32, f32),
    pub color: [f32; 4],
    pub text: Option<String>,
    pub hovered: bool,
    pub on_click: Option<Box<dyn FnMut()>>,
}
#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
impl UIElement {
    pub fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            size: (0.2, 0.2),
            color: [1.0, 1.0, 1.0, 1.0],
            text: None,
            hovered: false,
            on_click: None,
        }
    }
    pub fn new(
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 4],
        text: Option<String>,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            position, // Centered in the viewport
            size,     // 20% width and 10% height of the viewport
            color,    // Default to purple (fully opaque)
            text,
            hovered: false,
            on_click,
        }
    }
}

#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
pub struct UIManager {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub elements: Vec<UIElement>,
    pub num_indices: u32, // Total number of indices across all UI elements
    pub visibility: bool,
    pub bind_group: wgpu::BindGroup,
    pub font_texture: wgpu::Texture,
    pub font_sampler: wgpu::Sampler,
}

impl UIManager {
    pub fn update(&mut self, queue: &wgpu::Queue) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut current_index = 0;

        for element in &self.elements {
            // Existing rectangle processing code
            let (x, y) = element.position;
            let (w, h) = element.size;
            let color = element.color;
            let positions = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]];
            if let Some(text) = &element.text {
                if true {
                    // Background rectangle vertices
                    let positions = [
                        [x, y],
                        [x + w, y],
                        [x, y + h],
                        [x + w, y + h],
                    ];
                    for &pos in &positions {
                        vertices.push(Vertex {
                            position: pos,
                            uv: [0.0, 0.0], // Use default UV (font texture's [0,0] is white)
                            color: [1.0,1.0,1.0,0.8], // Background color
                        });
                    }
                    // Add indices for the rectangle
                    indices.extend_from_slice(&[
                        current_index + 0,
                        current_index + 1,
                        current_index + 2,
                        current_index + 1,
                        current_index + 3,
                        current_index + 2,
                    ]);
                    current_index += 4;
                }
				let leng:f32 = text.len() as f32; // would be just an u32 but at dividing that would crash
                let (char_width, char_height) = (w/leng,h/leng);
                for (i, c) in text.chars().enumerate() {
                    let (u_min, v_min, u_max, v_max) = get_uv(c);
                    let char_x = x + (i as f32) * char_width;
                    let char_y = y;
                    let positions = [
                        [char_x, char_y],
                        [char_x + char_width, char_y],
                        [char_x, char_y + char_height],
                        [char_x + char_width, char_y + char_height],
                    ];
                    let uvs = [
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_min, v_max],
                        [u_max, v_max],
                    ];
                    for j in 0..4 {
                        vertices.push(Vertex {
                            position: positions[j],
                            uv: uvs[j],
                            color,
                        });
                    }
                    indices.extend_from_slice(&[
                        current_index + 0,
                        current_index + 1,
                        current_index + 2,
                        current_index + 1,
                        current_index + 3,
                        current_index + 2,
                    ]);
                    current_index += 4;
                }
            } else {
                for &pos in &positions {
                    vertices.push(Vertex {
                        position: pos,
                        uv: [0.0, 0.0], // Default to white
                        color,
                    });
                }
                indices.extend_from_slice(&[
                    current_index + 0,
                    current_index + 1,
                    current_index + 2,
                    current_index + 1,
                    current_index + 3,
                    current_index + 2,
                ]);
                current_index += 4;
            }
        }
        // Write vertices to the GPU buffer
        let vertex_data = bytemuck::cast_slice(&vertices);
        queue.write_buffer(&self.vertex_buffer, 0, vertex_data);
        // Write indices to the GPU buffer
        let index_data = bytemuck::cast_slice(&indices);
        queue.write_buffer(&self.index_buffer, 0, index_data);
        // Update the total number of indices
        self.num_indices = indices.len() as u32;
    }

    pub fn add_ui_element(&mut self, element: self::UIElement) {
        self.elements.push(element);
    }

    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) -> Self {
        // Font Texture Setup
        let font_size = wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        };
        let font_data = [0xFFu8; (128 * 128 * 4) as usize]; // Placeholder white texture
        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            label: Some("Font Texture"),
            size: font_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &font_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(512),
                rows_per_image: None,
            },
            font_size,
        );

        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Bind Group Layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("font_bind_group_layout"),
        });

        let font_texture_view = font_texture.create_view(&Default::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&font_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&font_texture_view),
                },
            ],
            label: Some("font_bind_group"),
        });

        // Pipeline Layout Update
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });
        let ui_vertex_layout = vec![
            // Position attribute (location 0)
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            // UV attribute (location 1)
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            // Color attribute (location 2)
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
        ];

        let ui_vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ui_vertex_layout,
        };
        // UI Pipeline
        let shader: wgpu::ShaderModule =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("UI Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("ui_shader.wgsl").into()),
            });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&pipeline_layout), // Must include the texture bind group layout
            vertex: wgpu::VertexState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("vs_main"),
                buffers: &[ui_vertex_buffer_layout], // Use the corrected layout
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        // Fix the alpha component here:
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha, // Changed from Zero
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, // Changed from One
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None, // UI doesn't need depth
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // UI Buffers
        let vertex_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Vertex Buffer"),
            size: 1024 * std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Index Buffer"),
            size: 1024 * std::mem::size_of::<u32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            pipeline: ui_pipeline,
            elements: Vec::new(),
            num_indices: 0, // Initialize to 0
            visibility: true,
            bind_group,
            font_texture,
            font_sampler,
        }
    }
}

pub fn handle_ui_hover(
    current_state: &mut super::State,
    position: &winit::dpi::PhysicalPosition<f64>,
) {
    let (x, y) = (position.x as f32, position.y as f32);
    let (w, h) = (
        current_state.size.width as f32,
        current_state.size.height as f32,
    );

    // Convert to UI's normalized coordinates (-1.0 to 1.0)
    let norm_x = (2.0 * x / w) - 1.0;
    let norm_y = (2.0 * (h - y) / h) - 1.0; // Invert Y-axis

    for element in &mut current_state.ui_manager.elements {
        let (pos_x, pos_y) = element.position;
        let (size_w, size_h) = element.size;

        // Calculate element's bounds in normalized coordinates
        let right = pos_x + size_w;
        let top = pos_y + size_h;

        // Check if mouse is inside the element's bounds
        if norm_x >= pos_x && norm_x <= right && norm_y >= pos_y && norm_y <= top {
            element.hovered = true;
            element.color[2] = 0.5; // Green tint on hover
        } else {
            element.hovered = false;
            element.color[2] = 0.9; // Default color
        }
    }
}

pub fn handle_ui_click(current_state: &mut super::State) {
    for element in &mut current_state.ui_manager.elements {
        if element.hovered {
            // run the function
            if element.on_click.is_some() {
                element.on_click.as_deref_mut().unwrap()();
            }
        }
    }
}

pub fn get_uv(c: char) -> (f32, f32, f32, f32) {
    let code = c as u32;
    if code < 32 {
        return (0.0, 0.0, 0.0, 0.0); // Skip non-printable
    }
    let index = (code - 32) as usize;
    let grid_size = 16; // 16x16 grid in 128x128 texture
    let x = (index % grid_size) * 8;
    let y = (index / grid_size) * 8;
    let u_min = x as f32 / 128.0;
    let v_min = y as f32 / 128.0;
    let u_max = (x + 8) as f32 / 128.0;
    let v_max = (y + 8) as f32 / 128.0;
    (u_min, v_min, u_max, v_max)
}
