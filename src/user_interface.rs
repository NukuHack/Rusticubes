use image::GenericImageView;
use winit::keyboard::KeyCode as Key;

// Constants for common values
const DEFAULT_ALPHA: f32 = 0.9;
const HOVER_ALPHA: f32 = 0.5;
const MAX_INPUT_LENGTH: usize = 120;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[derive(Default)]
pub struct UIElement {
    pub id: usize, // Added ID field
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: [f32; 4],
    pub text: Option<String>,
    pub hovered: bool,
    pub is_input: bool,
    pub on_click: Option<Box<dyn FnMut()>>,
}

impl UIElement {
    pub const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, DEFAULT_ALPHA];
    pub const DEFAULT_SIZE: (f32, f32) = (0.2, 0.2);

    pub fn new(
        id: usize, // Added ID parameter
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: Option<String>,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            id,
            position,
            size,
            color: [color[0], color[1], color[2], DEFAULT_ALPHA],
            text,
            on_click,
            ..Default::default()
        }
    }

    pub fn new_input(
        id: usize, // Added ID parameter
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: Option<String>,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            id,
            is_input: true,
            ..Self::new(id, position, size, color, text, on_click)
        }
    }

    fn get_bounds(&self) -> (f32, f32, f32, f32) {
        let (x, y) = self.position;
        let (w, h) = self.size;
        (x, y, x + w, y + h)
    }
}

impl std::fmt::Debug for UIElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UIElement")
            .field("id", &self.id)
            .field("position", &self.position)
            .field("size", &self.size)
            .field("color", &self.color)
            .field("text", &self.text)
            .field("hovered", &self.hovered)
            .field("has_on_click", &self.on_click.is_some())
            .finish()
    }
}

pub struct UIManager {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub elements: Vec<UIElement>,
    pub focused_element: Option<usize>,
    pub num_indices: u32,
    pub visibility: bool,
    pub bind_group: wgpu::BindGroup,
    pub font_texture: wgpu::Texture,
    pub font_sampler: wgpu::Sampler,
    next_id: usize, // Counter for generating unique IDs
}

impl UIManager {
    pub fn update(&mut self, queue: &wgpu::Queue) {
        let (vertices, indices) = self.process_elements();

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        self.num_indices = indices.len() as u32;
    }

    fn process_elements(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::with_capacity(self.elements.len() * 4);
        let mut indices = Vec::with_capacity(self.elements.len() * 6);
        let mut current_index = 0u32;

        for element in &self.elements {
            if element.text.is_some() {
                self.process_text_element(element, &mut vertices, &mut indices, &mut current_index);
            } else {
                self.process_rect_element(element, &mut vertices, &mut indices, &mut current_index);
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
        // Add background rectangle
        self.add_rectangle(vertices, element.position, element.size, element.color);
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4;

        // Process text if present
        if let Some(text) = &element.text {
            let (x, y) = element.position;
            let (w, h) = element.size;
            let char_count = text.chars().count() as f32;
            let padding = 0.95;

            let (padded_w, padded_h) = (w * padding, h * padding);
            let (overhang_w, overhang_h) = (w - padded_w, h - padded_h);
            let char_size = (padded_w / char_count).min(padded_h);

            for (i, c) in text.chars().enumerate() {
                let (u_min, v_min, u_max, v_max) = self.get_texture_coordinates(c);
                let char_x = x + overhang_w / 2.0 + (i as f32) * char_size;
                let char_y = y + overhang_h / 2.0 + (padded_h - char_size) / 2.0;

                let positions = [
                    [char_x, char_y],
                    [char_x + char_size, char_y],
                    [char_x, char_y + char_size],
                    [char_x + char_size, char_y + char_size],
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

                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
        }
    }

    fn get_texture_coordinates(&self, c: char) -> (f32, f32, f32, f32) {
        let code = c as u32;
        if code < 32 || (code > 127 && code < 160) || code >= 32 + 51 * 15 {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let index = code - 32;
        let grid_wid = 16;
        let (cell_wid, cell_hei) = (15.0, 16.0);
        let (texture_wid, texture_hei) = (240.0, 768.0);

        let x = (index % grid_wid) as f32;
        let y = (index / grid_wid) as f32;

        (
            x * cell_wid / texture_wid,
            (y + 1.0) * cell_hei / texture_hei,
            (x + 1.0) * cell_wid / texture_wid,
            y * cell_hei / texture_hei,
        )
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
        vertices.extend([
            Vertex {
                position: [x, y + h],
                uv: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [x + w, y + h],
                uv: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [x, y],
                uv: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [x + w, y],
                uv: [0.0, 0.0],
                color,
            },
        ]);
    }

    fn rectangle_indices(&self, base: u32) -> [u32; 6] {
        [base, base + 1, base + 2, base + 1, base + 3, base + 2]
    }

    pub fn add_ui_element(&mut self, mut element: UIElement) {
        // Assign the next available ID if not already set
        if element.id == 0 {
            element.id = self.next_id;
            self.next_id += 1;
        }
        self.elements.push(element);
    }

    // Helper method to get an element by ID
    pub fn get_element(&self, id: usize) -> Option<&UIElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    // Helper method to get a mutable element by ID
    pub fn get_element_mut(&mut self, id: usize) -> Option<&mut UIElement> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    // Example method to get text from an input element by ID
    pub fn get_input_text(&self, id: usize) -> Option<&str> {
        self.elements
            .iter()
            .find(|e| e.id == id && e.is_input)
            .and_then(|e| e.text.as_deref())
    }

    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) -> Self {
        // Load font texture
        let (font_data, width, height) = Self::load_font_texture();

        let font_size = wgpu::Extent3d {
            width,
            height,
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
                bytes_per_row: Some(width * 4),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(include_str!(
                "ui_shader.wgsl"
            ))),
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Vertex Buffer"),
            size: 1024 * std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
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
            focused_element: None,
            num_indices: 0,
            visibility: true,
            bind_group,
            font_texture,
            font_sampler,
            next_id: 1, // Start IDs at 1 (0 is reserved for uninitialized)
        }
    }

    fn load_font_texture() -> (Vec<u8>, u32, u32) {
        let img = image::load_from_memory(super::FONT_MAP).expect("Failed to load font atlas");
        let (width, height) = img.dimensions();
        let rgba = img.into_rgba8();
        (rgba.into_raw(), width, height)
    }

    pub fn process_text_input(&mut self, focused_idx: usize, c: char) {
        if let Some(element) = self.elements.get_mut(focused_idx) {
            if !element.is_input {
                return;
            }

            if let Some(input_text) = &mut element.text {
                if input_text.len() >= MAX_INPUT_LENGTH || c.is_control() {
                    return;
                }
                input_text.push(c);
            }
        }
    }

    pub fn handle_backspace(&mut self, focused_idx: usize) {
        if let Some(element) = self.elements.get_mut(focused_idx) {
            if element.is_input {
                element.text.as_mut().map(|text| text.pop());
            }
        }
    }

    pub fn handle_enter(&mut self, _focused_idx: usize) {
        self.focused_element = None;
    }

    pub fn blur_current_element(&mut self) {
        self.focused_element = None;
    }

    pub fn toggle_visibility(&mut self) {
        self.visibility = !self.visibility;
    }
}

pub fn key_to_char(key: Key, shift: bool) -> Option<char> {
    match key {
        Key::KeyA => Some(if shift { 'A' } else { 'a' }),
        Key::KeyB => Some(if shift { 'B' } else { 'b' }),
        Key::KeyC => Some(if shift { 'C' } else { 'c' }),
        Key::KeyD => Some(if shift { 'D' } else { 'd' }),
        Key::KeyE => Some(if shift { 'E' } else { 'e' }),
        Key::KeyF => Some(if shift { 'F' } else { 'f' }),
        Key::KeyG => Some(if shift { 'G' } else { 'g' }),
        Key::KeyH => Some(if shift { 'H' } else { 'h' }),
        Key::KeyI => Some(if shift { 'I' } else { 'i' }),
        Key::KeyJ => Some(if shift { 'J' } else { 'j' }),
        Key::KeyK => Some(if shift { 'K' } else { 'k' }),
        Key::KeyL => Some(if shift { 'L' } else { 'l' }),
        Key::KeyM => Some(if shift { 'M' } else { 'm' }),
        Key::KeyN => Some(if shift { 'N' } else { 'n' }),
        Key::KeyO => Some(if shift { 'O' } else { 'o' }),
        Key::KeyP => Some(if shift { 'P' } else { 'p' }),
        Key::KeyQ => Some(if shift { 'Q' } else { 'q' }),
        Key::KeyR => Some(if shift { 'R' } else { 'r' }),
        Key::KeyS => Some(if shift { 'S' } else { 's' }),
        Key::KeyT => Some(if shift { 'T' } else { 't' }),
        Key::KeyU => Some(if shift { 'U' } else { 'u' }),
        Key::KeyV => Some(if shift { 'V' } else { 'v' }),
        Key::KeyW => Some(if shift { 'W' } else { 'w' }),
        Key::KeyX => Some(if shift { 'X' } else { 'x' }),
        Key::KeyY => Some(if shift { 'Y' } else { 'y' }),
        Key::KeyZ => Some(if shift { 'Z' } else { 'z' }),
        Key::Digit0 => Some(if shift { ')' } else { '0' }),
        Key::Digit1 => Some(if shift { '!' } else { '1' }),
        Key::Digit2 => Some(if shift { '@' } else { '2' }),
        Key::Digit3 => Some(if shift { '#' } else { '3' }),
        Key::Digit4 => Some(if shift { '$' } else { '4' }),
        Key::Digit5 => Some(if shift { '%' } else { '5' }),
        Key::Digit6 => Some(if shift { '^' } else { '6' }),
        Key::Digit7 => Some(if shift { '&' } else { '7' }),
        Key::Digit8 => Some(if shift { '*' } else { '8' }),
        Key::Digit9 => Some(if shift { '(' } else { '9' }),
        Key::Space => Some(' '),
        Key::Minus => Some(if shift { '_' } else { '-' }),
        Key::Equal => Some(if shift { '+' } else { '=' }),
        Key::BracketLeft => Some(if shift { '{' } else { '[' }),
        Key::BracketRight => Some(if shift { '}' } else { ']' }),
        Key::Backslash => Some(if shift { '|' } else { '\\' }),
        Key::Semicolon => Some(if shift { ':' } else { ';' }),
        Key::Quote => Some(if shift { '"' } else { '\'' }),
        Key::Comma => Some(if shift { '<' } else { ',' }),
        Key::Period => Some(if shift { '>' } else { '.' }),
        Key::Slash => Some(if shift { '?' } else { '/' }),
        _ => None,
    }
}

pub fn handle_ui_hover(state: &mut super::State, mouse_pos: &winit::dpi::PhysicalPosition<f64>) {
    let (norm_x, norm_y) = convert_mouse_position(state, mouse_pos);

    for element in &mut state.ui_manager.elements {
        if element.on_click.is_some() || element.text.is_some() {
            let (min_x, min_y, max_x, max_y) = element.get_bounds();
            element.hovered =
                norm_x >= min_x && norm_x <= max_x && norm_y >= min_y && norm_y <= max_y;
            element.color[3] = if element.hovered {
                HOVER_ALPHA
            } else {
                DEFAULT_ALPHA
            };
        }
    }
}

fn convert_mouse_position(
    state: &super::State,
    mouse_pos: &winit::dpi::PhysicalPosition<f64>,
) -> (f32, f32) {
    let x = mouse_pos.x as f32;
    let y = mouse_pos.y as f32;
    let width = state.size().width as f32;
    let height = state.size().height as f32;

    ((2.0 * x / width) - 1.0, (2.0 * (height - y) / height) - 1.0)
}

pub fn handle_ui_click(state: &mut super::State) {
    state.ui_manager.focused_element = None;

    for (index, element) in state.ui_manager.elements.iter_mut().enumerate() {
        if element.hovered {
            if element.is_input {
                state.ui_manager.focused_element = Some(index);
            } else if let Some(callback) = &mut element.on_click {
                callback();
            }
        }
    }
}

pub fn setup_ui(state: &mut super::State) {
    let elements = vec![
        UIElement::new(
            0,
            (0.45, 0.4),
            (0.5, 0.25),
            [0.7, 0.3, 0.3],
            Some("fill world w* chunks".to_string()),
            Some(Box::new(|| super::cube_extra::add_full_world())),
        ),
        UIElement::new(
            0,
            (0.6, -0.7),
            (0.2, 0.1),
            [1.0, 0.2, 0.1],
            Some("Close".to_string()),
            Some(Box::new(|| super::close_app())),
        ),
        UIElement::new(5, (0.0, -0.02), (0.02, 0.06), [0.1, 0.1, 0.1], None, None),
        UIElement::new(6, (-0.02, 0.0), (0.06, 0.02), [0.1, 0.1, 0.1], None, None),
    ];

    state.ui_manager.elements = elements;
}
