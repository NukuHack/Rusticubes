use std::borrow::Cow;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2], // Normalized device coordinates (-1.0 to 1.0)
    uv: [f32; 2],       // Texture coordinates (0.0 to 1.0)
    color: [f32; 4],    // RGBA color values (0.0-1.0)
}

#[allow(dead_code, unused)]
#[derive(Default)]
pub struct UIElement {
    pub position: (f32, f32), // Center position in normalized coordinates
    pub size: (f32, f32),     // Width and height in normalized coordinates
    pub color: [f32; 4],      // Base color of the element
    pub text: String,         // Optional text content
    pub hovered: bool,        // Hover state
    pub on_click: Option<Box<dyn FnMut()>>, // Click callback
}
#[allow(dead_code, unused)]
impl UIElement {
    pub const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
    pub const DEFAULT_SIZE: (f32, f32) = (0.2, 0.2);

    pub fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            size: Self::DEFAULT_SIZE,
            color: Self::DEFAULT_COLOR,
            text: String::new(), // Use an empty string as the default text
            hovered: false,
            on_click: None,
        }
    }

    pub fn new(
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: String,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            position,
            size,
            color: [color[0], color[1], color[2], 0.9],
            text,
            on_click,
            ..Default::default()
        }
    }

    // Helper for calculating element bounds
    fn get_bounds(&self) -> (f32, f32, f32, f32) {
        let (x, y) = self.position;
        let (w, h) = self.size;
        (x, y, x + w, y + h)
    }
}
impl std::fmt::Debug for UIElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UIElement")
            .field("position", &self.position)
            .field("size", &self.size)
            .field("color", &self.color)
            .field("text", &self.text)
            .field("hovered", &self.hovered)
            .field("has_on_click", &self.on_click.is_some())
            .finish()
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
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut current_index = 0u32;
        let empty_string = String::new();

        for element in &self.elements {
            if &empty_string == &element.text {
                self.process_rect_element(element, &mut vertices, &mut indices, &mut current_index);
            } else {
                self.process_text_element(element, &mut vertices, &mut indices, &mut current_index);
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
        let (x, y): (f32, f32) = element.position;
        let (w, h): (f32, f32) = element.size;
        let char_count = element.text.chars().count() as f32;

        // Add background rectangle
        self.add_rectangle(vertices, element.position, element.size, element.color);
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4;

        // Calculate padding and character size
        let padding: f32 = 0.95;
        let (padded_w, padded_h): (f32, f32) = (w * padding, h * padding);
        let (overhang_w, overhang_h): (f32, f32) = (w - padded_w, h - padded_h);
        let char_size: f32 = (padded_w / char_count).min(padded_h); // Determine the maximum possible size per character

        // Process each character
        for (i, c) in element.text.chars().enumerate() {
            let (u_min, v_min, u_max, v_max) = self.get_texture_coordinates(c);

            // Calculate horizontal position (already centered horizontally as analyzed)
            let char_x = x + overhang_w / 2.0 + (i as f32) * char_size;

            // Calculate vertical position to center vertically within the padded area
            let char_y = y + overhang_h / 2.0 + (padded_h - char_size) / 2.0;

            // Define character vertices and UVs with correct height
            let positions = [
                [char_x, char_y],                         // Top-left
                [char_x + char_size, char_y],             // Top-right
                [char_x, char_y + char_size],             // Bottom-left
                [char_x + char_size, char_y + char_size], // Bottom-right
            ];
            let uvs = [
                [u_min, v_min],
                [u_max, v_min],
                [u_min, v_max],
                [u_max, v_max],
            ];

            // Add vertices and indices as before
            for j in 0..4 {
                vertices.push(Vertex {
                    position: positions[j],
                    uv: uvs[j],
                    color: element.color,
                });
            }
            indices.extend(self.rectangle_indices(*current_index));
            *current_index += 4;
        }
    }
    /*
    fn get_texture_coordinates2(&self, c: char) -> (f32, f32, f32, f32) {
        let code = c as u32;
        if code < 32 || (code > 127 && code < 160) || code >= 32 + 51 * 15 {
            return (0.0, 0.0, 0.0, 0.0); // Non-printable characters return zero coordinates
        }

        let index: u32 = code - 32;
        // Adjust the code to start at 32
        let grid_wid: u32 = 51;
        let (cell_wid, cell_hei): (f32, f32) = (15.0, 16.0);
        let (texture_wid, texture_hei): (f32, f32) = (765.0, 282.0);
        // Calculate the column and row in the grid
        let (x, y): (f32, f32) = ((index % grid_wid) as f32, (index / grid_wid) as f32);
        // Compute texture coordinates
        let u_min: f32 = (x) * cell_wid / texture_wid;
        let v_min: f32 = (y) * cell_hei / texture_hei;
        let u_max: f32 = (x + 1.0f32) * cell_wid / texture_wid;
        let v_max: f32 = (y + 1.0f32) * cell_hei / texture_hei;
        // Texture coordinates reversed vertically (common in some frameworks)
        (u_min, v_max, u_max, v_min)
    }*/
    fn get_texture_coordinates(&self, c: char) -> (f32, f32, f32, f32) {
        let code = c as u32;
        if code < 32 || (code > 127 && code < 160) || code >= 32 + 51 * 15 {
            return (0.0, 0.0, 0.0, 0.0); // Non-printable characters return zero coordinates
        }
        let index: u32 = code - 32;
        // Adjust the code to start at 32
        let grid_wid: u32 = 16;
        let (cell_wid, cell_hei): (f32, f32) = (8.0, 8.0);
        let (texture_wid, texture_hei): (f32, f32) = (128.0, 128.0);
        // Calculate the column and row in the grid
        let (x, y): (f32, f32) = ((index % grid_wid) as f32, (index / grid_wid) as f32);
        // Compute texture coordinates
        let u_min: f32 = (x) * cell_wid / texture_wid;
        let v_min: f32 = (y) * cell_hei / texture_hei;
        let u_max: f32 = (x + 1.0f32) * cell_wid / texture_wid;
        let v_max: f32 = (y + 1.0f32) * cell_hei / texture_hei;
        // Texture coordinates reversed vertically (common in some frameworks)
        (u_min, v_max, u_max, v_min)
    }

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
        let font_data = {
            let current_dir: std::path::PathBuf =
                std::env::current_dir().expect("Failed to get current directory");

            let raw_path: &str = r"font.png"; // raw : font.png ; good : bescii-chars.png

            let full_path: std::path::PathBuf = current_dir.join("resources").join(raw_path);
            let path: &str = full_path.to_str().expect("Path contains invalid UTF-8");

            let img = image::open(path).expect("Failed to load font atlas");
            let rgba = img.into_rgba8();
            rgba.into_raw()
        };

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
                bytes_per_row: Some(128 * 4),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::from(include_str!("ui_shader.wgsl"))),
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
    let (norm_x, norm_y) = convert_mouse_position(state, mouse_pos);

    for element in &mut state.ui_manager.elements {
        let (min_x, min_y, max_x, max_y) = element.get_bounds();
        element.hovered = norm_x >= min_x && norm_x <= max_x && norm_y >= min_y && norm_y <= max_y;

        element.color[3] = if element.hovered { 0.5 } else { 0.9 };
    }
}
fn convert_mouse_position(
    state: &super::State,
    mouse_pos: &winit::dpi::PhysicalPosition<f64>,
) -> (f32, f32) {
    let (x, y) = (mouse_pos.x as f32, mouse_pos.y as f32);
    let (width, height) = (state.size.width as f32, state.size.height as f32);

    let norm_x = (2.0 * x / width) - 1.0;
    let norm_y = (2.0 * (height - y) / height) - 1.0;

    (norm_x, norm_y)
}

// Click handling
pub fn handle_ui_click(state: &mut super::State) {
    for element in &mut state.ui_manager.elements {
        if element.hovered && element.on_click.is_some() {
            element.on_click.as_mut().unwrap()();
        }
    }
}
