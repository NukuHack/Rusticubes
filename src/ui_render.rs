use super::ui_element::{UIElement, UIElementType, Vertex};
use image::GenericImageView;

pub struct UIRenderer {
    pub bind_group: wgpu::BindGroup,
    pub font_texture: wgpu::Texture,
    pub font_sampler: wgpu::Sampler,
}

impl UIRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
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

        Self {
            bind_group,
            font_texture,
            font_sampler,
        }
    }

    fn load_font_texture() -> (Vec<u8>, u32, u32) {
        // Assuming FONT_MAP is available in the parent module
        let img = image::load_from_memory(super::FONT_MAP).expect("Failed to load font atlas");
        let (width, height) = img.dimensions();
        let rgba = img.into_rgba8();
        (rgba.into_raw(), width, height)
    }

    pub fn process_elements(&self, elements: &[UIElement]) -> (Vec<Vertex>, Vec<u32>) {
        // Sort elements by z-index for proper rendering order
        let mut sorted_elements: Vec<&UIElement> = elements.iter().filter(|e| e.visible).collect();
        sorted_elements.sort_by_key(|e| e.z_index);

        let mut vertices = Vec::with_capacity(sorted_elements.len() * 8); // Account for borders
        let mut indices = Vec::with_capacity(sorted_elements.len() * 12);
        let mut current_index = 0u32;

        for element in sorted_elements {
            // Render border first (if it exists) so it appears behind the element
            if element.border_width > 0.0 {
                self.process_border(element, &mut vertices, &mut indices, &mut current_index);
            }

            // Render the main element
            match element.element_type {
                UIElementType::Checkbox => {
                    self.process_checkbox(element, &mut vertices, &mut indices, &mut current_index);
                }
                _ => {
                    if element.text.is_some() {
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
            }
        }

        (vertices, indices)
    }

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

        // Create border as a larger rectangle behind the main element
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
        &self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        // Render checkbox background
        self.process_rect_element(element, vertices, indices, current_index);

        // Render checkmark if checked
        if element.checked {
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
                [0.2, 0.7, 0.2, 1.0], // Green checkmark
            );
            indices.extend(self.rectangle_indices(*current_index));
            *current_index += 4;
        }

        // Render label if present
        if let Some(text) = &element.text {
            let label_element = UIElement {
                position: (
                    element.position.0 + element.size.0 + 0.01,
                    element.position.1,
                ),
                size: (text.len() as f32 * 0.01, element.size.1),
                text: Some(text.clone()),
                color: [0.0, 0.0, 0.0, 1.0], // Black text
                on_click: None,
                ..*element
            };
            self.process_text_element(&label_element, vertices, indices, current_index);
        }
    }

    fn process_text_element(
        &self,
        element: &UIElement,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        current_index: &mut u32,
    ) {
        // Add background rectangle for input fields and buttons
        match element.element_type {
            UIElementType::InputField | UIElementType::Button => {
                self.add_rectangle(vertices, element.position, element.size, element.color);
                indices.extend(self.rectangle_indices(*current_index));
                *current_index += 4;
            }
            _ => {}
        }

        // Process text if present
        if let Some(text) = &element.text {
            let (x, y) = element.position;
            let (w, h) = element.size;
            let char_count = text.chars().count() as f32;
            let padding = 0.95;

            let (padded_w, padded_h) = (w * padding, h * padding);
            let (overhang_w, overhang_h) = (w - padded_w, h - padded_h);
            let char_size = (padded_w / char_count).min(padded_h);

            // Determine text color based on element type
            let text_color = match element.element_type {
                UIElementType::InputField => [0.0, 0.0, 0.0, 1.0], // Black text for input
                UIElementType::Button => [1.0, 1.0, 1.0, 1.0],     // White text for buttons
                UIElementType::Label => element.color,             // Use element color for labels
                _ => [0.0, 0.0, 0.0, 1.0],                         // Default black
            };

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
                        color: text_color,
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
}
