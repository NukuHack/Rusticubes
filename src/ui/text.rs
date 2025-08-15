
use crate::{
	get_bytes,
	ui::{
		element::UIElement,
		manager::{FocusState, UIManager},
		render::UIRenderer,
	},
	utils::color::Color,
};
use glam::Vec2;
use rusttype::{Font, Scale, point};
use image::{ImageBuffer, Rgba};
use winit::keyboard::KeyCode::{self as Key, *};

impl UIManager {
	pub fn handle_key_input_on_input_field(&mut self, key: Key, input_str: &str) -> bool {
		let Some(element) = self.get_focused_element_mut() else { return false; };
		if !(element.visible && element.enabled) { return false; }

		if !element.is_input() {
			if key != Escape { return false; }
			self.clear_focused_state();
			return false;
		}
		match key {
			Backspace => {
				let Some(text_mut) = element.get_text_mut() else { return false; };
				
				if text_mut.is_empty() { return false; }

				text_mut.pop();
			},
			Enter | Escape => self.clear_focused_state(),
			_ => {
				let Some(text_mut) = element.get_text_mut() else { return false; };
				
				if text_mut.len() >= 256 { return false; }
				
				text_mut.push_str(input_str);
			}
		}
		return true;
	}
}



const ELLIPSIS: &str = "...";

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TruncateMode {
	/// Position-based truncation
	/// will remove any overflowing text and replace with "..." at the position
	Head,    // "…ple text" (keep end)
	Tail,    // "Sample te…" (keep start)
	Body,    // "Sam…ext" (keep both ends)
	/// Dynamic behavior - relative to ui size
	/// this just will make sure it fits the ui
	Shrink,  // Reduce font size to fit
	Grow,    // Expand font size to fit (if needed)
	ShrinkGrow, // Hybrid (shrink or grow)
}
impl TruncateMode {
	pub const LEFT:Self = Self::Head; // this will make it so whatever you write it will be shown, but old text not
	pub const RIGHT:Self = Self::Tail; // this will make it so first part of the sentence will be shown mainly
	pub const CENTER:Self = Self::Body;
	pub const fn default() -> Self {
		Self::RIGHT
	}
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AlignMode {
	/// Position-based "Push"
	/// this will not truncate anything just overflow from the text input (the variation here states which part will be aligned to the element correctly)
	Lt, Lm, Lb, // Left top, Left middle, Left bottom
	Ct, Cm, Cb, // Center ...
	Rt, Rm, Rb, // Right ...
}
impl AlignMode {
	pub const CENTER:Self = Self::Cm;
	pub const fn default() -> Self {
		Self::CENTER // Center middle is the default
	}
}

impl UIElement {
	#[inline] pub fn handle_input_clicked(&mut self, _x:f32, _y:f32) -> FocusState {
		FocusState::input(self.id)
	}
}

impl UIRenderer {
	#[inline] pub fn change_font(&mut self, path: String) {
		self.font = Font::try_from_vec(get_bytes!(path)).expect("Failed to load font");
		self.clear_text();
	}
	
	#[inline] pub const fn set_pixel_ratio(&mut self, ratio: f32) {
		self.pixel_ratio = ratio.max(10.0).min(0.5);
	}

	/// Renders text to a GPU texture with specified formatting
	pub fn render_text_to_texture(&self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, element_size: Vec2, color: Color, truncate_mode: TruncateMode, align_mode: AlignMode) -> wgpu::Texture {
		let pixel_ratio = if element_size.x + element_size.y < 0.2 {
			self.pixel_ratio * 3.0
		} else {
			self.pixel_ratio
		};

		let target_width_px = (element_size.x * 100.0 * pixel_ratio) as u32;
		let target_height_px = (element_size.y * 100.0 * pixel_ratio) as u32;
		let font_size = target_height_px as f32 * 0.8;
		let scale = Scale::uniform(font_size);
		let v_metrics = self.font.v_metrics(scale);

		let text_width = self.calculate_text_width(text, scale);
		let (final_text, final_scale) = if text_width > target_width_px as f32 * 0.95 {
			self.truncate_text(text, scale, target_width_px as f32, &truncate_mode)
		} else {
			(text.to_string(), scale)
		};

		let final_width = self.calculate_text_width(&final_text, final_scale);
		let padding = (font_size * 0.05).ceil() as u32;
		let text_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;

		let (width, text_start_x) = {
			let texture_width = target_width_px
				.max((final_width.ceil() as u32 + padding * 2).max(1));
			let start_x = self.calculate_x(&final_text, final_scale, texture_width as f32, align_mode);
			(texture_width, start_x)
		};

		let height = (text_height + padding * 2).max(1);
		let mut image = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));

		let adjusted_glyphs: Vec<_> = self
			.font
			.layout(
				&final_text,
				final_scale,
				point(text_start_x, v_metrics.ascent + padding as f32),
			)
			.collect();

		for glyph in adjusted_glyphs {
			let Some(bounding_box) = glyph.pixel_bounding_box() else {
				continue;
			};

			glyph.draw(|x, y, v| {
				let x = x as i32 + bounding_box.min.x;
				let y = y as i32 + bounding_box.min.y;
				if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
					image.put_pixel(
						x as u32,
						y as u32,
						Rgba(color.with_a((v * 200.0) as u8).to_arr()),
					);
				}
			});
		}

		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some("Text Texture"),
			size: wgpu::Extent3d {
				width,
				height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8Unorm,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			view_formats: &[],
		});

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
			wgpu::Extent3d {
				width,
				height,
				depth_or_array_layers: 1,
			},
		);

		texture
	}
	fn calculate_x(&self, text: &str, scale: Scale, texture_width: f32, align: AlignMode) -> f32 {
		let text_width = self.calculate_text_width(text, scale);
		
		match align {
			AlignMode::Lt | AlignMode::Lm | AlignMode::Lb => 0.0, // Align start to left edge
			AlignMode::Ct | AlignMode::Cm | AlignMode::Cb => (texture_width - text_width) / 2.0, // Center the text
			AlignMode::Rt | AlignMode::Rm | AlignMode::Rb => texture_width - text_width, // Align end to right edge
		}
	}

	fn truncate_text(&self, text: &str, scale: Scale, max_width: f32, mode: &TruncateMode) -> (String, Scale) {
		match mode {
			TruncateMode::Head => (self.truncate_head(text, scale, max_width), scale),
			TruncateMode::Tail => (self.truncate_tail(text, scale, max_width), scale),
			TruncateMode::Body => (self.truncate_body(text, scale, max_width), scale),
			TruncateMode::Shrink => self.shrink_to_fit(text, scale, max_width),
			TruncateMode::Grow => self.grow_to_fit(text, scale, max_width),
			TruncateMode::ShrinkGrow => self.shrink_grow_to_fit(text, scale, max_width),
		}
	}

	fn truncate_tail(&self, text: &str, scale: Scale, max_width: f32) -> String {
		let ellipsis_width = self.calculate_text_width(ELLIPSIS, scale);

		if ellipsis_width >= max_width {
			return ELLIPSIS.to_string();
		}

		let mut result = String::new();
		let mut current_width = 0.0;

		for c in text.chars() {
			let advance = self.font.glyph(c).scaled(scale).h_metrics().advance_width;
			if current_width + advance + ellipsis_width > max_width {
				result.push_str(ELLIPSIS);
				break;
			}
			result.push(c);
			current_width += advance;
		}

		if result.len() < text.len() && !result.ends_with(ELLIPSIS) {
			result.push_str(ELLIPSIS);
		}

		result
	}

	fn truncate_head(&self, text: &str, scale: Scale, max_width: f32) -> String {
		let ellipsis_width = self.calculate_text_width(ELLIPSIS, scale);

		if ellipsis_width >= max_width {
			return ELLIPSIS.to_string();
		}

		let chars: Vec<char> = text.chars().collect();
		let mut result = String::new();
		let mut current_width = ellipsis_width;

		for &c in chars.iter().rev() {
			let advance = self.font.glyph(c).scaled(scale).h_metrics().advance_width;
			if current_width + advance > max_width {
				break;
			}
			result.insert(0, c);
			current_width += advance;
		}

		if result.len() >= text.len() {
			return result;
		}

		format!("{}{}", ELLIPSIS, result)
	}

	fn truncate_body(&self, text: &str, scale: Scale, max_width: f32) -> String {
		let ellipsis_width = self.calculate_text_width(ELLIPSIS, scale);

		if ellipsis_width >= max_width {
			return ELLIPSIS.to_string();
		}

		let available_width = max_width - ellipsis_width;
		let chars: Vec<char> = text.chars().collect();

		if chars.is_empty() {
			return String::new();
		}

		let mut left_part = String::new();
		let mut right_part = String::new();
		let mut left_width = 0.0;
		let mut right_width = 0.0;
		let mut left_idx = 0;
		let mut right_idx = chars.len() - 1;

		while left_idx <= right_idx {
			// Try adding to left
			if left_idx <= right_idx {
				let advance = self.font.glyph(chars[left_idx]).scaled(scale).h_metrics().advance_width;
				if left_width + advance <= available_width / 2.0 {
					left_part.push(chars[left_idx]);
					left_width += advance;
					left_idx += 1;
				} else if right_width < available_width / 2.0 {
					// If left can't fit, try right
					let right_advance = self.font.glyph(chars[right_idx]).scaled(scale).h_metrics().advance_width;
					if right_width + right_advance > available_width - left_width {
						break;
					}

					right_part.insert(0, chars[right_idx]);
					right_width += right_advance;
					if right_idx == 0 {
						break;
					}
					right_idx -= 1;
				} else {
					break;
				}
			}

			// Try adding to right
			if left_idx <= right_idx {
				let advance = self.font.glyph(chars[right_idx]).scaled(scale).h_metrics().advance_width;
				if right_width + advance <= available_width / 2.0 {
					right_part.insert(0, chars[right_idx]);
					right_width += advance;
					if right_idx == 0 {
						break;
					}
					right_idx -= 1;
				} else if left_width < available_width / 2.0 {
					// If right can't fit, try left
					let left_advance = self.font.glyph(chars[left_idx]).scaled(scale).h_metrics().advance_width;
					if left_width + left_advance > available_width - right_width {
						break;
					}

					left_part.push(chars[left_idx]);
					left_width += left_advance;
					left_idx += 1;
				} else {
					break;
				}
			}
		}

		if left_idx <= right_idx {
			return text.to_string();
		}

		format!("{}{}{}", left_part, ELLIPSIS, right_part)
	}

	fn shrink_to_fit(&self, text: &str, scale: Scale, max_width: f32) -> (String, Scale) {
		let current_width = self.calculate_text_width(text, scale);

		if current_width <= max_width {
			return (text.to_string(), scale);
		}

		// Binary search for optimal scale
		let mut min_scale = 1.0;
		let mut max_scale = scale.x;
		let mut optimal_scale = scale.x;

		for _ in 0..8 {
			let mid_scale = (min_scale + max_scale) / 2.0;
			let test_scale = Scale::uniform(mid_scale);
			let width = self.calculate_text_width(text, test_scale);

			if width > max_width {
				max_scale = mid_scale;
			} else {
				min_scale = mid_scale;
				optimal_scale = mid_scale;
			}
		}

		(text.to_string(), Scale::uniform(optimal_scale))
	}

	fn grow_to_fit(&self, text: &str, scale: Scale, max_width: f32) -> (String, Scale) {
		let current_width = self.calculate_text_width(text, scale);

		if current_width >= max_width {
			return (text.to_string(), scale);
		}

		// Binary search for optimal scale
		let mut min_scale = scale.x;
		let mut max_scale = scale.x * 20.0;
		let mut optimal_scale = scale.x;

		for _ in 0..12 {
			let mid_scale = (min_scale + max_scale) / 2.0;
			let test_scale = Scale::uniform(mid_scale);
			let width = self.calculate_text_width(text, test_scale);

			if width < max_width {
				min_scale = mid_scale;
				optimal_scale = mid_scale;
			} else {
				max_scale = mid_scale;
			}
		}

		(text.to_string(), Scale::uniform(optimal_scale))
	}

	fn shrink_grow_to_fit(&self, text: &str, scale: Scale, max_width: f32) -> (String, Scale) {
		let current_width = self.calculate_text_width(text, scale);

		if (current_width - max_width).abs() < 1.0 {
			return (text.to_string(), scale);
		}

		if current_width > max_width {
			return self.shrink_to_fit(text, scale, max_width);
		}

		self.grow_to_fit(text, scale, max_width)
	}

	fn calculate_text_width(&self, text: &str, scale: Scale) -> f32 {
		self.font
			.layout(text, scale, point(0.0, 0.0))
			.map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
			.last()
			.unwrap_or(0.0)
	}
}
