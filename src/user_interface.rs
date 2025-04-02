use std::borrow::Cow;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2], // Normalized device coordinates (-1.0 to 1.0)
    uv: [f32; 2],       // Texture coordinates (0.0 to 1.0)
    color: [f32; 4],    // RGBA color values (0.0-1.0)
}

#[allow(dead_code, unused)]
pub struct UIElement {
    pub position: (f32, f32), // Center position in normalized coordinates
    pub size: (f32, f32),     // Width and height in normalized coordinates
    pub color: [f32; 4],      // Base color of the element
    pub r#type: String,       // the type like : "rect" or "text"
    pub text: String,         // Optional text content
    pub hovered: bool,        // Hover state
    pub on_click: Option<Box<dyn FnMut()>>, // Click callback
}
#[allow(dead_code, unused)]
impl UIElement {
    pub fn default() -> Self {
        Self {
            position: (0.0, 0.0),        // Centered in the viewport
            size: (0.2, 0.2),            // 20% width and 10% height of the viewport
            color: [1.0, 1.0, 1.0, 1.0], // Default to purple (fully opaque
            r#type: "rect".to_string(),
            text: "None".to_string(),
            hovered: false,
            on_click: None,
        }
    }
    pub fn new_rect(
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 4],
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            position,
            size,
            color,
            r#type: "rect".to_string(),
            text: "None".to_string(),
            hovered: false,
            on_click,
        }
    }
    pub fn new_text(
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 4],
        text: String,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            position,
            size,
            color,
            r#type: "text".to_string(),
            text,
            hovered: false,
            on_click,
        }
    }
    pub fn new(
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 4],
        r#type: String,
        text: String,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            position,
            size,
            color,
            r#type,
            text,
            hovered: false,
            on_click,
        }
    }
}

#[allow(dead_code, unused)]
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
        let (vertices, indices) = self.process_elements();

        // Update GPU buffers
        let vertex_data = bytemuck::cast_slice(&vertices);
        queue.write_buffer(&self.vertex_buffer, 0, vertex_data);
        let index_data = bytemuck::cast_slice(&indices);
        queue.write_buffer(&self.index_buffer, 0, index_data);
        self.num_indices = indices.len() as u32;
    }

    // Helper functions for clarity
    fn process_elements(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut current_index = 0;

        for element in &self.elements {
            match &element.r#type {
                val if *val == String::from("text") => {
                    self.process_text_element(
                        &element,
                        &mut vertices,
                        &mut indices,
                        &mut current_index,
                    );
                    // Add 4 (rectangle) + (text.len() *4) vertices
                    current_index += element.text.len() as u32 * 4;
                }
                val if *val == String::from("rect") => {
                    self.process_rect_element(
                        &element,
                        &mut vertices,
                        &mut indices,
                        &mut current_index,
                    );
                }
                String { .. } => println!("unimplemented ui type {:?}", element.r#type),
            }
        }

        (vertices, indices)
    }

    fn process_text_element(
        &self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        let (x, y) = element.position;
        let (w, h) = element.size;
        let char_count = element.text.chars().count() as f32;

        // Background rectangle
        let background_color = [1.0, 1.0, 1.0, 0.8];
        self.add_rectangle(vertices, element.position, element.size, background_color);
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4; // Add 4 to skip the rectangle's vertices
                             // Text characters
        let (char_width, char_height) = (w / char_count, h / char_count);
        for (i, c) in element.text.chars().enumerate() {
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
                    color: element.color,
                });
            }
            let base = *current_index + (i as u32 * 4);
            indices.extend(self.rectangle_indices(base));
        }
    }

    // Helper methods
    fn process_rect_element(
        &self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        self.add_rectangle(vertices, element.position, element.size, element.color);
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4;
    }

    fn add_rectangle(
        &self,
        vertices: &mut Vec<Vertex>,
        (x, y): (f32, f32),
        (w, h): (f32, f32),
        color: [f32; 4],
    ) {
        // Positions are relative to the bottom-left corner aka the position of the element
        let positions = [
            [x, y + h],     // Top-left
            [x + w, y + h], // Top-right
            [x, y],         // Bottom-left
            [x + w, y],     // Bottom-right
        ];

        for j in 0..4 {
            vertices.push(Vertex {
                position: positions[j],
                uv: [0.0, 0.0],
                color,
            });
        }
    }

    fn rectangle_indices(&self, base: u32) -> [u32; 6] {
        [base + 0, base + 1, base + 2, base + 1, base + 3, base + 2]
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
        let font_data = [0xFFu8; (128 * 128 * 4) as usize]; //generate_font_atlas(); // Generate or load your font atlas data
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
                source: wgpu::ShaderSource::Wgsl(Cow::from(
                    r#"
                        struct VertexInput {
                            @location(0) position: vec2<f32>,
                            @location(1) uv: vec2<f32>,
                            @location(2) color: vec4<f32>,
                        };

                        struct VertexOutput {
                            @location(0) uv: vec2<f32>,
                            @location(1) color: vec4<f32>,
                            // Mark the position with @builtin(position)
                            @builtin(position) position: vec4<f32>,
                        };

                        @vertex
                        fn vs_main(in: VertexInput) -> VertexOutput {
                            var out: VertexOutput;
                            out.uv = in.uv;
                            out.color = in.color;
                            // Assign to the position field instead of gl_Position
                            out.position = vec4<f32>(in.position, 0.0, 1.0);
                            return out;
                        }

                        @group(0) @binding(0) var font_sampler: sampler;
                        @group(0) @binding(1) var font_texture: texture_2d<f32>;

                        struct FragmentInput {
                            @location(0) uv: vec2<f32>,
                            @location(1) color: vec4<f32>,
                        };

                        @fragment
                        fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
                            let sampled_color = textureSample(font_texture, font_sampler, in.uv);
                            return in.color * sampled_color;
                        }
                    "#,
                )),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

pub fn handle_ui_hover(state: &mut super::State, mouse_pos: &winit::dpi::PhysicalPosition<f64>) {
    let (x, y) = (mouse_pos.x as f32, mouse_pos.y as f32);
    let (width, height) = (state.size.width as f32, state.size.height as f32);

    let norm_x = (2.0 * x / width) - 1.0;
    let norm_y = (2.0 * (height - y) / height) - 1.0; // Invert Y-axis

    for element in &mut state.ui_manager.elements {
        let (pos_x, pos_y) = element.position;
        let (w, h) = element.size;

        if norm_x >= pos_x && norm_x <= pos_x + w && norm_y >= pos_y && norm_y <= pos_y + h {
            element.hovered = true;
            // Apply hover color
            element.color[2] = 0.5; // Green tint
        } else {
            element.hovered = false;
            element.color[2] = 0.9; // Default blue
        }
    }
}

// Click handling
pub fn handle_ui_click(state: &mut super::State) {
    for element in &mut state.ui_manager.elements {
        if element.hovered {
            if let Some(callback) = &mut element.on_click {
                callback();
            }
        }
    }
}

// UV coordinate calculation
pub fn get_uv(c: char) -> (f32, f32, f32, f32) {
    let code = c as u32;
    if code < 32 {
        return (0.0, 0.0, 0.0, 0.0); // Non-printable characters
    }
    let grid_size = 16; // 16x16 grid in 128x128 texture
    let index = (code - 32) as usize;
    let (x, y) = (index % grid_size, index / grid_size);
    let cell_size = 8.0; // Each cell is 8x8 pixels
    let u_min = (x as f32 * cell_size) / 128.0;
    let v_min = (y as f32 * cell_size) / 128.0;
    let u_max = (x as f32 + 1.0) * cell_size / 128.0;
    let v_max = (y as f32 + 1.0) * cell_size / 128.0;
    (u_min, v_min, u_max, v_max)
}

pub fn generate_font_atlas() -> Vec<u8> {
    const FONT_WIDTH: usize = 128;
    const FONT_HEIGHT: usize = 128;
    const CELL_SIZE: usize = 8; // Each character cell is 8x8 pixels

    let mut font_data = vec![0u8; FONT_WIDTH * FONT_HEIGHT * 4]; // RGBA buffer

    // Draw test pattern for each character (ASCII 32 to 126)
    for code in 32..=126 {
        let index = (code - 32) as usize;
        let x = (index % 16) * CELL_SIZE; // 16 columns in 128px width
        let y = (index / 16) * CELL_SIZE;

        // Draw a 4x4 square in the center of the cell
        let center_x = x + (CELL_SIZE / 2 - 2);
        let center_y = y + (CELL_SIZE / 2 - 2);

        for dy in 0..4 {
            for dx in 0..4 {
                let px = center_x + dx;
                let py = center_y + dy;

                // Calculate pixel position (row-major order)
                let offset = (py * FONT_WIDTH + px) * 4;
                font_data[offset] = 255; // R
                font_data[offset + 1] = 255; // G
                font_data[offset + 2] = 255; // B
                font_data[offset + 3] = 255; // A (opaque)
            }
        }
    }

    font_data
}
