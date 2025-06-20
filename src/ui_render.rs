use rusttype::{Font, Scale, point};
use image::{ImageBuffer,Rgba};
use std::collections::HashMap;
use super::ui_element::{UIElement, UIElementData};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub struct UIRenderer {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub font_sampler: wgpu::Sampler,
    font: Font<'static>,
    text_textures: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    image_textures: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>, // Add this line
    default_bind_group: wgpu::BindGroup,
    pixel_ratio: f32, // For high DPI support
}

impl UIRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Load font from embedded bytes or file
        let font_data = crate::get_bytes!("calibri.ttf"); // C:\Windows\Fonts\..
        let font = Font::try_from_vec(font_data.clone()).expect("Failed to load font");

        // Create sampler for font rendering
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
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

        // Create default texture
        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Texture"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Fill with white pixel
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &default_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );

        let texture_view = default_texture.create_view(&Default::default());
        let default_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&font_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
            label: Some("default_bind_group"),
        });

        Self {
            bind_group_layout,
            font_sampler,
            font,
            text_textures: HashMap::new(),
            image_textures: HashMap::new(), // Add this line
            default_bind_group,
            pixel_ratio: 4.0,
        }
    }

    pub fn change_font(&mut self, path: String) {
        let font_data = crate::get_bytes!(path.clone()); // C:\Windows\Fonts\..
        let font = Font::try_from_vec(font_data.clone()).expect("Failed to load font");

        self.font = font;
        self.text_textures.clear();
    }

    pub fn set_pixel_ratio(&mut self, ratio: f32) {
        self.pixel_ratio = ratio.max(6.0).min(0.5);
    }

    pub fn process_elements(
        &mut self,
        elements: &[UIElement],
    ) -> (Vec<Vertex>, Vec<u32>) {
        let mut sorted_elements: Vec<&UIElement> = elements.iter().filter(|e| e.visible).collect();
        sorted_elements.sort_by_key(|e| e.z_index);

        let mut vertices = Vec::with_capacity(sorted_elements.len() * 8);
        let mut indices = Vec::with_capacity(sorted_elements.len() * 12);
        let mut current_index = 0u32;

        for element in sorted_elements {
            if element.border_width > 0.0 {
                self.process_border(element, &mut vertices, &mut indices, &mut current_index);
            }

            match &element.data {
                UIElementData::Image { .. } => {
                    self.process_image_element(element, &mut vertices, &mut indices, &mut current_index);
                }
                UIElementData::Checkbox { .. } => {
                    self.process_checkbox(element, &mut vertices, &mut indices, &mut current_index);
                }
                UIElementData::InputField { .. } | UIElementData::Button { .. } | UIElementData::Label { .. }  => {
                    if element.get_text().is_some() {
                        self.process_text_element(
                            element,
                            &mut vertices,
                            &mut indices,
                            &mut current_index,
                        );
                    } else {
                        self.process_rect_element(
                            element,
                            &mut vertices,
                            &mut indices,
                            &mut current_index,
                        );
                    }
                }
                UIElementData::Panel { .. } | UIElementData::Divider { .. } => {
                    self.process_rect_element(
                        element,
                        &mut vertices,
                        &mut indices,
                        &mut current_index,
                    );
                }
            }
        }

        (vertices, indices)
    }

    fn process_text_element(
        &mut self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        let state = super::config::get_state();
        
        // First process the background if needed (not for label)
        match &element.data {
            UIElementData::InputField { .. } | UIElementData::Button { .. } => {
                self.add_rectangle(vertices, element.position, element.size, element.color);
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
            _ => {}
        }

        if let Some(text) = element.get_text() {
            let texture_key = format!("{}_{}_{}", text, element.size.0, element.size.1);
            
            if !self.text_textures.contains_key(&texture_key) {
                let texture = self.render_text_to_texture(
                    state.device(),
                    state.queue(),
                    text,
                    element.size.1 * 100.0 * self.pixel_ratio, // Scale based on height and pixel ratio
                    element.color,
                );
                
                let texture_view = texture.create_view(&Default::default());
                let bind_group = state.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&self.font_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                    ],
                    label: Some("font_bind_group"),
                });
                
                self.text_textures.insert(texture_key.clone(), (texture, bind_group));
            }

            if let Some((texture, _)) = self.text_textures.get(&texture_key) {
                let (pos_x, pos_y) = element.position;
                let (siz_w, siz_h) = element.size;
                let pixel_to_unit = 1.0 / (100.0 * self.pixel_ratio);
                
                let w = texture.width() as f32 * pixel_to_unit;
                let h = texture.height() as f32 * pixel_to_unit;
                
                // Center the text in the element
                let x = pos_x + (siz_w - w) / 2.0;
                let y = pos_y + (siz_h - h) / 2.0;
                // Correct vertex positions and UVs
                let positions = [
                    [x, y],         // top-left
                    [x + w, y],     // top-right
                    [x, y + h],     // bottom-left
                    [x + w, y + h], // bottom-right
                ];
                let uvs = [
                     [0.0, 1.0], [1.0, 1.0],[0.0, 0.0], [1.0, 0.0],
                ];
                for j in 0..4 {
                    vertices.push(Vertex {
                        position: positions[j],
                        uv: uvs[j],
                        color: element.color, // Use white color and let texture provide the color
                    });
                }
                
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
        }
    }

    fn render_text_to_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        scale: f32,
        color: [f32; 4],
    ) -> wgpu::Texture {        
        let scale = Scale::uniform(scale);
        let v_metrics = self.font.v_metrics(scale);

        // Layout the text
        let glyphs: Vec<_> = self.font.layout(text, scale, point(0.0, v_metrics.ascent)).collect();

        // Calculate text dimensions with padding
        let padding = (scale.x * 0.2).ceil() as u32;
        let width = (glyphs.iter().rev()
            .map(|g| {
                let pos = g.position().x;
                let advance = g.unpositioned().h_metrics().advance_width;
                pos + advance
            })
            .next()
            .unwrap_or(0.0)
            .ceil() as u32) + padding * 2;
        
        let height = ((v_metrics.ascent - v_metrics.descent).ceil() as u32) + padding * 2;

        // Ensure minimum size
        let width = width.max(1);
        let height = height.max(1);

        // Create an image buffer with transparency
        let mut image = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
        
        // Convert color to u8
        let [r, g, b, a] = color;
        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        let a = (a * 255.0) as u8;

        // Render each glyph with padding offset
        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let x = x as i32 + bounding_box.min.x + padding as i32;
                    let y = y as i32 + bounding_box.min.y + padding as i32;
                    if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                        let alpha = (v * a as f32) as u8;
                        // Use premultiplied alpha for better quality
                        let premultiplied = |c: u8| ((c as f32 * v).round() as u8);
                        image.put_pixel(
                            x as u32, 
                            y as u32, 
                            Rgba([
                                premultiplied(r),
                                premultiplied(g),
                                premultiplied(b),
                                alpha,
                            ])
                        );
                    }
                });
            }
        }

        // Create texture with mipmaps for better quality at small sizes
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, // Consider increasing for better quality at small sizes
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, // Use sRGB for better color handling
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload texture data
        let raw_data = image.into_raw();
        
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &raw_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );

        texture
    }


    // Add this new method to process image elements
    fn process_image_element(
        &mut self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        if let UIElementData::Image { path } = &element.data {
            let state = super::config::get_state();
            let image_key = format!("{}_{}_{}", path, element.size.0, element.size.1);
            
            // Create or get cached image texture
            if !self.image_textures.contains_key(&image_key) {
                let texture = self.create_image_texture(
                    state.device(),
                    state.queue(),
                    path.to_string(),
                );
                
                let texture_view = texture.create_view(&Default::default());
                let bind_group = state.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&self.font_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                    ],
                    label: Some("image_bind_group"),
                });
                
                self.image_textures.insert(image_key, (texture, bind_group));
            }

            // Add vertices for the image quad - FIXED: Match the vertex order used in add_rectangle
            let (x, y) = element.position;
            let (w, h) = element.size;
            // Correct vertex positions and UVs
            let positions = [
                [x, y],         // top-left
                [x + w, y],     // top-right
                [x, y + h],     // bottom-left
                [x + w, y + h], // bottom-right
            ];
            let uvs = [
                 [0.0, 1.0], [1.0, 1.0],[0.0, 0.0], [1.0, 0.0],
            ];
            for j in 0..4 {
                vertices.push(Vertex {
                    position: positions[j],
                    uv: uvs[j],
                    color: element.color, // Use white color and let texture provide the color
                });
            }
            
            indices.extend(self.rectangle_indices(*current_index));
            *current_index += 4;
        }
    }

    // Add this method to create image textures
    fn create_image_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: String,
    ) -> wgpu::Texture {
        let Some((rgba,width,height)) = super::resources::load_image_from_bytes(path.to_string()) else { panic!() };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, //Rgba8UnormSrgb
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload the pixel data
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
                bytes_per_row: Some(4 * width), // 4 bytes per pixel (RGBA)
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );

        texture
    }
}

impl UIRenderer {
    
    fn process_border(
        &self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        let (x, y) = element.position;
        let (w, h) = element.size;
        let border_width = element.border_width;

        let border_x = x - border_width;
        let border_y = y - border_width;
        let border_w = w + 2.0 * border_width;
        let border_h = h + 2.0 * border_width;

        self.add_rectangle(
            vertices,
            (border_x, border_y),
            (border_w, border_h),
            element.border_color,
        );
        indices.extend(self.rectangle_indices(*current_index));
        *current_index += 4;
    }

    fn process_checkbox(
        &mut self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
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

                self.add_rectangle(
                    vertices,
                    (check_x, check_y),
                    (check_w, check_h),
                    [0.2, 0.7, 0.2, 1.0],
                );
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
        }

        if let Some(text) = element.get_text() {
            let label_element = UIElement {
                position: (
                    element.position.0 + element.size.0 + 0.01,
                    element.position.1,
                ),
                size: (text.len() as f32 * 0.01, element.size.1),
                data: UIElementData::Label {
                    text: text.to_string(),
                },
                color: [0.0, 0.0, 0.0, 1.0],
                ..*element
            };
            self.process_text_element(&label_element, vertices, indices, current_index);
        }
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

}


impl UIRenderer {
    // insainly long rendering function made by "https://claude.ai/chat/" 4.0 to be exact
    pub fn render<'a>(
        &'a self,
        ui_manager: &super::ui_manager::UIManager,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        render_pass.set_pipeline(&ui_manager.pipeline);
        render_pass.set_vertex_buffer(0, ui_manager.vertex_buffer.slice(..));
        render_pass.set_index_buffer(ui_manager.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        let mut index_offset = 0;
        
        // Get sorted elements (same as in process_elements)
        let mut sorted_elements: Vec<&UIElement> = ui_manager.elements.iter().filter(|e| e.visible).collect();
        sorted_elements.sort_by_key(|e| e.z_index);
        
        for element in sorted_elements {
            // Draw border first (if it exists)
            if element.border_width > 0.0 {
                render_pass.set_bind_group(0, &self.default_bind_group, &[]);
                render_pass.draw_indexed(
                    index_offset..(index_offset + 6),
                    0, 
                    0..1
                );
                index_offset += 6;
            }
            
            // Draw background/element body
            match &element.data {
                UIElementData::Image { path } => {
                    let image_key = format!("{}_{}_{}", path, element.size.0, element.size.1);
                    // Draw image with its texture
                    if let Some((_, bind_group)) = self.image_textures.get(&image_key) {
                        render_pass.set_bind_group(0, bind_group, &[]);
                        render_pass.draw_indexed(
                            index_offset..(index_offset + 6),
                            0,
                            0..1
                        );
                    }
                    index_offset += 6;
                }
                UIElementData::Checkbox { checked, .. } => {
                    // Draw checkbox background
                    render_pass.set_bind_group(0, &self.default_bind_group, &[]);
                    render_pass.draw_indexed(
                        index_offset..(index_offset + 6),
                        0, 
                        0..1
                    );
                    index_offset += 6;
                    
                    // Draw checkmark if checked
                    if *checked {
                        render_pass.draw_indexed(
                            index_offset..(index_offset + 6),
                            0, 
                            0..1
                        );
                        index_offset += 6;
                    }
                    
                    // Draw checkbox label text (if any)
                    if let Some(text) = element.get_text() {
                        let texture_key = format!("{}_{}_{}", text, element.size.0, element.size.1);
                        if let Some((_, bind_group)) = self.text_textures.get(&texture_key) {
                            render_pass.set_bind_group(0, bind_group, &[]);
                            render_pass.draw_indexed(
                                index_offset..(index_offset + 6),
                                0,
                                0..1
                            );
                        }
                        index_offset += 6;
                    }
                }
                UIElementData::InputField { .. } | UIElementData::Button { .. } => {
                    // Draw background rectangle
                    render_pass.set_bind_group(0, &self.default_bind_group, &[]);
                    render_pass.draw_indexed(
                        index_offset..(index_offset + 6),
                        0, 
                        0..1
                    );
                    index_offset += 6;
                    
                    // Draw text if element has text
                    if let Some(text) = element.get_text() {
                        let texture_key = format!("{}_{}_{}", text, element.size.0, element.size.1);
                        if let Some((_, bind_group)) = self.text_textures.get(&texture_key) {
                            render_pass.set_bind_group(0, bind_group, &[]);
                            render_pass.draw_indexed(
                                index_offset..(index_offset + 6),
                                0,
                                0..1
                            );
                        }
                        index_offset += 6;
                    }
                }
                UIElementData::Panel { .. } | UIElementData::Divider { .. } => {
                    // Draw the panel/divider rectangle
                    render_pass.set_bind_group(0, &self.default_bind_group, &[]);
                    render_pass.draw_indexed(
                        index_offset..(index_offset + 6),
                        0, 
                        0..1
                    );
                    index_offset += 6;
                }
                UIElementData::Label { .. } => {
                    // Draw text if element has text
                    if let Some(text) = element.get_text() {
                        let texture_key = format!("{}_{}_{}", text, element.size.0, element.size.1);
                        if let Some((_, bind_group)) = self.text_textures.get(&texture_key) {
                            render_pass.set_bind_group(0, bind_group, &[]);
                            render_pass.draw_indexed(
                                index_offset..(index_offset + 6),
                                0,
                                0..1
                            );
                        }
                        index_offset += 6;
                    }
                }
            }
        }
    }
}