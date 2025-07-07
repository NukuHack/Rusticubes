
use crate::config;
use crate::ext::rs;
use crate::ui::element::{UIElement, UIElementData};
use rusttype::{Font, Scale, point};
use image::{ImageBuffer, Rgba};
use std::collections::HashMap;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub packed_data: u32,
}

impl Vertex {
    #[inline] pub fn new(position: [f32; 2], color: [u8; 4]) -> Self {
        Self { position, packed_data: pack_color_u8(color) }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Uint32 },
            ],
        }
    }
}

#[inline] fn pack_color_u8(color: [u8; 4]) -> u32 {
    (color[0] as u32) << 24 | (color[1] as u32) << 16 | (color[2] as u32) << 8 | color[3] as u32
}

pub struct UIRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    font_sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    font: Font<'static>,
    text_textures: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    image_textures: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    animation_textures: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    default_bind_group: wgpu::BindGroup,
    pixel_ratio: f32,
}

impl UIRenderer {
    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    #[inline]
    pub fn font_sampler(&self) -> &wgpu::Sampler {
        &self.font_sampler
    }
    #[inline]
    pub fn uniform_buffer(&self) -> &wgpu::Buffer {
        &self.uniform_buffer
    }
    #[inline]
    pub fn uniform_bind_group(&self) -> &wgpu::BindGroup {
        &self.uniform_bind_group
    }
    #[inline]
    pub fn uniform_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.uniform_bind_group_layout
    }
    #[inline] pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let font = Font::try_from_vec(crate::get_bytes!("calibri.ttf")).expect("Failed to load font");
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, 
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false }, count: None },
            ],
            label: Some("ui_bind_group_layout"),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<u32>() as u64 * 2,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, 
                    has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
        });

        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Texture"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &default_texture, mip_level: 0, 
                origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );

        let texture_view = default_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });

        let default_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&font_sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&texture_view) },
            ],
            label: Some("default_bind_group"),
        });

        Self {
            bind_group_layout, font_sampler, uniform_buffer, uniform_bind_group,
            uniform_bind_group_layout, font, text_textures: HashMap::new(),
            image_textures: HashMap::new(), animation_textures: HashMap::new(),
            default_bind_group, pixel_ratio: 4.0,
        }
    }
    
    #[inline] pub fn change_font(&mut self, path: String) {
        self.font = Font::try_from_vec(crate::get_bytes!(path)).expect("Failed to load font");
        self.text_textures.clear();
    }
    
    #[inline] pub fn set_pixel_ratio(&mut self, ratio: f32) {
        self.pixel_ratio = ratio.max(10.0).min(0.5);
    }
    
    #[inline] pub fn process_elements(&mut self, elements: &[UIElement]) -> (Vec<Vertex>, Vec<u32>) {
        let mut sorted_elements: Vec<_> = elements.iter().filter(|e| e.visible).collect();
        sorted_elements.sort_by_key(|e| e.z_index);
        let mut vertices = Vec::with_capacity(sorted_elements.len() * 8);
        let mut indices = Vec::with_capacity(sorted_elements.len() * 12);
        let mut current_index = 0u32;

        for element in sorted_elements {
            if element.border_width > 0.0 {
                self.process_border(element, &mut vertices, &mut indices, &mut current_index);
            }
            match &element.data {
                UIElementData::Image { .. } => self.process_image_element(element, &mut vertices, &mut indices, &mut current_index),
                UIElementData::Animation { .. } => self.process_animation_element(element, &mut vertices, &mut indices, &mut current_index),
                UIElementData::Checkbox { .. } => self.process_checkbox(element, &mut vertices, &mut indices, &mut current_index),
                UIElementData::InputField { .. } | UIElementData::Button { .. } | UIElementData::Label { .. } => {
                    if element.get_text().is_some() {
                        self.process_text_element(element, &mut vertices, &mut indices, &mut current_index);
                    } else {
                        self.process_rect_element(element, &mut vertices, &mut indices, &mut current_index);
                    }
                }
                _ => self.process_rect_element(element, &mut vertices, &mut indices, &mut current_index),
            }
        }
        (vertices, indices)
    }

    #[inline] fn process_text_element(&mut self, element: &UIElement, vertices: &mut Vec<Vertex>, 
        indices: &mut Vec<u32>, current_index: &mut u32) {
        let state = config::get_state();
        
        match &element.data {
            UIElementData::InputField { .. } | UIElementData::Button { .. } => {
                self.add_rectangle(vertices, element.position, element.size, element.color);
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
            _ => {}
        }

        if let Some(text) = element.get_text() {
            let texture_key = format!("{}_{:?}", text, element.color);
            if !self.text_textures.contains_key(&texture_key) {
                let texture = self.render_text_to_texture(state.device(), state.queue(), text, element.size, element.get_text_color());
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
                let bind_group = state.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.font_sampler) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&texture_view) },
                    ],
                    label: Some("font_bind_group"),
                });
                self.text_textures.insert(texture_key.clone(), (texture, bind_group));
            }

            if let Some((texture, _)) = self.text_textures.get(&texture_key) {
                let (x, y) = element.position;
                let (w, h) = element.size;
                let pixel_to_unit = 1.0 / (100.0 * self.pixel_ratio);
                let tex_w = texture.width() as f32 * pixel_to_unit;
                let tex_h = texture.height() as f32 * pixel_to_unit;
                let x = x + (w - tex_w) / 2.0;
                let y = y + (h - tex_h) / 2.0;
                let positions = [[x, y], [x + tex_w, y], [x, y + tex_h], [x + tex_w, y + tex_h]];
                
                for j in 0..4 {
                    vertices.push(Vertex::new(positions[j], [255, 255, 255, 255]));
                }
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
        }
    }

    fn render_text_to_texture(&self, device: &wgpu::Device, queue: &wgpu::Queue, 
        text: &str, element_size: (f32, f32), [r, g, b, a]: [u8; 4]) -> wgpu::Texture {
        let target_width_px = (element_size.0 * 100.0 * self.pixel_ratio) as u32;
        let target_height_px = (element_size.1 * 100.0 * self.pixel_ratio) as u32;
        let font_size = target_height_px as f32 * 0.8;
        let scale = Scale::uniform(font_size);
        let v_metrics = self.font.v_metrics(scale);
        
        // Calculate text width by summing up advances
        let text_width = self.font.layout(text, scale, point(0.0, 0.0))
            .fold(0.0, |width, g| width + g.unpositioned().h_metrics().advance_width);
        
        let (final_text, final_width) = if text_width > target_width_px as f32 * 0.9 {
            let truncated = self.truncate_text(text, scale, target_width_px as f32 * 1.1);
            let truncated_width = self.font.layout(&truncated, scale, point(0.0, 0.0))
                .fold(0.0, |width, g| width + g.unpositioned().h_metrics().advance_width);
            (truncated, truncated_width)
        } else {
            (text.to_string(), text_width)
        };
        
        let padding = (font_size * 0.05).ceil() as u32;
        let text_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let width = (final_width.ceil() as u32 + padding * 2).max(1);
        let height = (text_height + padding * 2).max(1);

        let mut image = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
        let text_start_x = (width as f32 - final_width) / 2.0;
        let adjusted_glyphs: Vec<_> = self.font.layout(&final_text, scale, 
            point(text_start_x, v_metrics.ascent + padding as f32)).collect();

        for glyph in adjusted_glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let x = x as i32 + bounding_box.min.x;
                    let y = y as i32 + bounding_box.min.y;
                    if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                        let alpha = (v * a as f32).round() as u8;
                        image.put_pixel(x as u32, y as u32, 
                            Rgba([r, g, b, alpha]));
                    }
                });
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let raw_data = image.into_raw();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, 
                origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &raw_data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        texture
    }

    fn truncate_text(&self, text: &str, scale: Scale, max_width: f32) -> String {
        let mut result = String::new();
        let ellipsis = "...";
        let ellipsis_width = self.font.layout(ellipsis, scale, point(0.0, 0.0))
            .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
            .last().unwrap_or(0.0);
        
        if ellipsis_width >= max_width { return ellipsis.to_string(); }
        
        let mut current_width = 0.0;
        for c in text.chars() {
            let advance = self.font.glyph(c).scaled(scale).h_metrics().advance_width;
            if current_width + advance + ellipsis_width > max_width {
                result.push_str(ellipsis);
                break;
            }
            result.push(c);
            current_width += advance;
        }
        result
    }

    #[inline] fn process_image_element(&mut self, element: &UIElement, vertices: &mut Vec<Vertex>, 
        indices: &mut Vec<u32>, current_index: &mut u32) {
        if let UIElementData::Image { path } = &element.data {
            let state = config::get_state();
            let image_key = format!("{}_{}", path, element.color.map(|c| c.to_string()).join("|"));
            
            if !self.image_textures.contains_key(&image_key) {
                let texture = self.create_image_texture(state.device(), state.queue(), path.to_string());
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
                let bind_group = state.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.font_sampler) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&texture_view) },
                    ],
                    label: Some("image_bind_group"),
                });
                self.image_textures.insert(image_key, (texture, bind_group));
            }

            let (x, y) = element.position;
            let (w, h) = element.size;
            let positions = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]];
            for j in 0..4 {
                vertices.push(Vertex::new(positions[j], element.color));
            }
            indices.extend(self.rectangle_indices(*current_index));
            *current_index += 4;
        }
    }

    fn create_image_texture(&self, device: &wgpu::Device, queue: &wgpu::Queue, path: String) -> wgpu::Texture {
        let (rgba, width, height) = rs::load_image_from_path(path.to_string()).unwrap();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, 
                origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        texture
    }

    #[inline] fn process_animation_element(&mut self, element: &UIElement, vertices: &mut Vec<Vertex>, 
        indices: &mut Vec<u32>, current_index: &mut u32) {
        if let UIElementData::Animation {frames,..} = &element.data {
            let state = config::get_state();
            let animation_key = frames.join("|");
            
            if !self.animation_textures.contains_key(&animation_key) {
                if let Some((texture, bind_group)) = self.create_animation_texture_array(state.device(), state.queue(), frames) {
                    self.animation_textures.insert(animation_key.clone(), (texture, bind_group));
                }
            }

            let (x, y) = element.position;
            let (w, h) = element.size;
            let positions = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]];
            for j in 0..4 {
                vertices.push(Vertex::new(positions[j], element.color));
            }
            indices.extend(self.rectangle_indices(*current_index));
            *current_index += 4;
        }
    }

    fn create_animation_texture_array(&self, device: &wgpu::Device, queue: &wgpu::Queue, 
        frames: &[String]) -> Option<(wgpu::Texture, wgpu::BindGroup)> {
        if frames.is_empty() { return None; }
        let (_, width, height) = rs::load_image_from_path(frames[0].clone())?;
        let layer_count = frames.len() as u32;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("animation_texture_array"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: layer_count },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (i, frame_path) in frames.iter().enumerate() {
            if let Some((rgba, _, _)) = rs::load_image_from_path(frame_path.clone()) {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, 
                        origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 }, aspect: wgpu::TextureAspect::All },
                    &rgba,
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
                    wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                );
            }
        }

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.font_sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&texture_view) },
            ],
            label: Some("animation_bind_group"),
        });
        Some((texture, bind_group))
    }

    #[inline] fn process_border(&self, element: &UIElement, vertices: &mut Vec<Vertex>, 
        indices: &mut Vec<u32>, current_index: &mut u32) {
        let (x, y) = element.position;
        let (w, h) = element.size;
        let border_width = element.border_width;
        let border_x = x - border_width;
        let border_y = y - border_width;
        let border_w = w + 2.0 * border_width;
        let border_h = h + 2.0 * border_width;
        self.add_rectangle(vertices, (border_x, border_y), (border_w, border_h), element.border_color);
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4;
    }

    #[inline] fn process_checkbox(&mut self, element: &UIElement, vertices: &mut Vec<Vertex>, 
        indices: &mut Vec<u32>, current_index: &mut u32) {
        self.process_rect_element(element, vertices, indices, current_index);
        if let UIElementData::Checkbox { checked, .. } = &element.data {
            if *checked {
                let (x, y) = element.position;
                let (w, h) = element.size;
                let padding = 0.2;
                let check_x = x + w * padding;
                let check_y = y + h * padding;
                let check_w = w * (1.0 - 2.0 * padding);
                let check_h = h * (1.0 - 2.0 * padding);
                self.add_rectangle(vertices, (check_x, check_y), (check_w, check_h), [50, 120, 50, 255]);
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
        }
        if let Some(text) = element.get_text() {
            let label_element = UIElement {
                position: (element.position.0 + element.size.0 + 0.01, element.position.1),
                size: (text.len() as f32 * 0.01, element.size.1),
                data: UIElementData::Label { text: text.to_string(), text_color: None },
                color: [0, 0, 0, 255],
                ..*element
            };
            self.process_text_element(&label_element, vertices, indices, current_index);
        }
    }

    #[inline] fn process_rect_element(&self, element: &UIElement, vertices: &mut Vec<Vertex>, 
        indices: &mut Vec<u32>, current_index: &mut u32) {
        self.add_rectangle(vertices, element.position, element.size, element.color);
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4;
    }

    #[inline] fn add_rectangle(&self, vertices: &mut Vec<Vertex>, (x, y): (f32, f32), 
        (w, h): (f32, f32), color: [u8; 4]) {
        let packed_color = pack_color_u8(color);
        vertices.extend([
            Vertex{position: [x, y + h], packed_data: packed_color},
            Vertex{position: [x + w, y + h], packed_data: packed_color},
            Vertex{position: [x, y], packed_data: packed_color},
            Vertex{position: [x + w, y], packed_data: packed_color},
        ]);
    }

    #[inline] fn rectangle_indices(&self, base: u32) -> [u32; 6] {
        [base, base + 1, base + 2, base + 1, base + 3, base + 2]
    }

    #[inline] pub fn render<'a>(&'a self, ui_manager: &crate::ui::manager::UIManager, 
        r_pass: &mut wgpu::RenderPass<'a>) {
        r_pass.set_pipeline(&ui_manager.pipeline);
        r_pass.set_vertex_buffer(0, ui_manager.vertex_buffer.slice(..));
        r_pass.set_index_buffer(ui_manager.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        r_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        let mut i_off = 0;
        let mut sorted_elements: Vec<_> = ui_manager.elements.iter().filter(|e| e.visible).collect();
        sorted_elements.sort_by_key(|e| e.z_index);
        
        for element in sorted_elements {
            if element.border_width > 0.0 {
                r_pass.set_bind_group(0, &self.default_bind_group, &[]);
                r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                i_off += 6;
            }
            match &element.data {
                UIElementData::Image { path } => {
                    let image_key = format!("{}_{}", path, element.color.map(|c| c.to_string()).join("|"));
                    if let Some((_, bind_group)) = self.image_textures.get(&image_key) {
                        r_pass.set_bind_group(0, bind_group, &[]);
                        r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                    }
                    i_off += 6;
                },
                UIElementData::Animation { frames, current_frame, elapsed_time, smooth_transition, blend_delay, frame_duration, .. } => {
                    let animation_key = frames.join("|");
                    if let Some((_, bind_group)) = self.animation_textures.get(&animation_key) {
                        let frame_count = frames.len() as u32;
                        let next_frame = if *smooth_transition { (*current_frame + 1) % frame_count } else { *current_frame };
                        let packed_frames = (*current_frame & 0xFFFF) | ((next_frame & 0xFFFF) << 16);
                        let raw_progress = ((elapsed_time / frame_duration) * 100.0) as u32;
                        let packed_progress = (raw_progress & 0xFFFF) | ((blend_delay & 0xFFFF) << 16);
                        config::get_state().queue().write_buffer(
                            &self.uniform_buffer, 0,
                            bytemuck::cast_slice(&[packed_frames, packed_progress])
                        );
                        r_pass.set_bind_group(0, bind_group, &[]);
                        r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                    }
                    i_off += 6;
                },
                UIElementData::Checkbox { checked, .. } => {
                    r_pass.set_bind_group(0, &self.default_bind_group, &[]);
                    r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                    i_off += 6;
                    if *checked {
                        r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                        i_off += 6;
                    }
                    if let Some(text) = element.get_text() {
                        let texture_key = format!("{}_{:?}", text, element.color);
                        if let Some((_, bind_group)) = self.text_textures.get(&texture_key) {
                            r_pass.set_bind_group(0, bind_group, &[]);
                            r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                        }
                        i_off += 6;
                    }
                },
                UIElementData::InputField { .. } | UIElementData::Button { .. } => {
                    r_pass.set_bind_group(0, &self.default_bind_group, &[]);
                    r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                    i_off += 6;
                    if let Some(text) = element.get_text() {
                        let texture_key = format!("{}_{:?}", text, element.color);
                        if let Some((_, bind_group)) = self.text_textures.get(&texture_key) {
                            r_pass.set_bind_group(0, bind_group, &[]);
                            r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                        }
                        i_off += 6;
                    }
                },
                UIElementData::Panel { .. } | UIElementData::Divider { .. } => {
                    r_pass.set_bind_group(0, &self.default_bind_group, &[]);
                    r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                    i_off += 6;
                },
                UIElementData::Label { .. } => {
                    if let Some(text) = element.get_text() {
                        let texture_key = format!("{}_{:?}", text, element.color);
                        if let Some((_, bind_group)) = self.text_textures.get(&texture_key) {
                            r_pass.set_bind_group(0, bind_group, &[]);
                            r_pass.draw_indexed(i_off..(i_off + 6), 0, 0..1);
                        }
                        i_off += 6;
                    }
                },
                _ => todo!(),
            }
        }
    }
}