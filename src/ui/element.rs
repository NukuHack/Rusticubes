
use crate::utils::color::{Color, Border};
use std::{cell::RefCell, sync::Arc };
use crate::ext::config::ElementStyle;
use glam::Vec2;
use crate::utils::string::MutStr;

type Callback = Arc<RefCell<dyn FnMut() + 'static>>;

#[derive(Clone)]
pub enum UIElementData {
	Panel,
	Label { text: MutStr, text_color: Color },
	Button { text: MutStr, text_color: Color },
	MultiStateButton { states: Vec<MutStr>, text_color: Color, current_state: usize },
	InputField { text: MutStr, text_color: Color, placeholder: MutStr },
	Checkbox { text: MutStr, text_color: Color, checked: bool },
	Image { path: MutStr },
	Animation {
		frames: Vec<MutStr>, current_frame: u32, frame_duration: f32, elapsed_time: f32,
		looping: bool, playing: bool, blend_delay: Option<u32>
	},
	Slider {
		min_value: f32, text_color: Color, max_value: f32, current_value: f32, step: Option<f32>
	},
}
impl UIElementData {
	#[inline] pub const fn default() -> Self { UIElementData::Panel }
}
#[derive(Clone)]
pub enum ElementData {
	Text(String),
	Number(f32),
	None,
}
impl ElementData {
	#[inline] pub fn text(&self) -> Option<String> {
		match self {
			ElementData::Text(s) if !s.is_empty() => Some(s.clone()),
			_ => None,
		}
	}
	#[inline] pub fn num(&self) -> Option<f32> {
		match self {
			ElementData::Number(s) => Some(*s),
			_ => None,
		}
	}
}
#[derive(Clone)]
pub struct UIElement {
	// Identity
	pub id: usize,

	// Hierarchy
	pub parent: Option<(usize, Vec2)>,

	// Layout
	pub position: Vec2,
	pub size: Vec2,
	//pub min_size: Vec2,
	//pub max_size: Vec2,
	//pub margin: Rect,
	//pub padding: Rect,
	pub vertical: bool,
	pub z_index: i32,

	// Appearance
	pub color: Color,
	pub border: Border,
	//pub background: Option<Color>,  // Could support gradients/textures

	// State
	pub hovered: bool,
	//pub focused: bool,
	//pub active: bool,
	pub visible: bool,
	pub enabled: bool,

	// Behavior
	pub event_handler: Option<Callback>,

	// Content
	pub data: UIElementData,
}

impl UIElement {
	#[inline] pub const fn default() -> Self {
		Self::new(0, UIElementData::default())
	}
	// Element creation
	#[inline]
	pub const fn new(id: usize, data: UIElementData) -> Self {
		Self {
			id,
			data,
			position: Vec2::new(0.0, 0.0),
			size: Vec2::new(0.0, 0.0),
			color: Color::DEF_COLOR,
			parent: None,
			hovered: false,
			z_index: 0,
			visible: true,
			border: Border::NONE,
			enabled: true,
			vertical: false,
			event_handler: None,
		}
	}
	#[inline]
	pub fn panel(id: usize) -> Self {
		Self::new(id, UIElementData::default())
	}
	#[inline]
	pub fn label(id: usize, text: MutStr) -> Self {
		Self::new(id, UIElementData::Label { text, text_color: Color::DEF_COLOR })
	}
	#[inline]
	pub fn button(id: usize, text: MutStr) -> Self {
		Self::new(id, UIElementData::Button { text, text_color: Color::DEF_COLOR })
	}
	#[inline]
	pub fn input(id: usize) -> Self {
		Self::new(id, UIElementData::InputField { text: MutStr::default(), text_color: Color::DEF_COLOR, placeholder: MutStr::default() })
	}
	#[inline]
	pub fn checkbox(id: usize) -> Self {
		Self::new(id, UIElementData::Checkbox { text: MutStr::default(), text_color: Color::DEF_COLOR, checked: false })
	}
	#[inline]
	pub fn image(id: usize, path: MutStr) -> Self {
		Self::new(id, UIElementData::Image { path })
	}
	#[inline]
	pub fn animation(id: usize, frames: Vec<MutStr>) -> Self {
		Self::new(id, UIElementData::Animation {
			frames, current_frame: 0, frame_duration: 1.0, elapsed_time: 0.0,
			looping: true, playing: true, blend_delay: Some(20)
		})
	}
	#[inline]
	pub fn multi_state_button(id: usize, states: Vec<MutStr>) -> Self {
		Self::new(id, UIElementData::MultiStateButton {
			states, text_color: Color::DEF_COLOR,
			current_state: 0
		})
	}
	#[inline]
	pub fn slider(id: usize, min_value: f32, max_value: f32) -> Self {
		Self::new(id, UIElementData::Slider {
			min_value, max_value, //vertical: false,
			text_color: Color::DEF_COLOR,
			current_value: min_value,
			step: None,
		})
	}
}

macro_rules! builder_method {
	($name:ident, $field:ident, $type:ty) => {
		pub const fn $name(mut self, $field: $type) -> Self {
			self.$field = $field;
			self
		}
	};
}
macro_rules! setter_method {
	($name:ident, $field:ident, $type:ty) => {
		pub const fn $name(&mut self, $field: $type) {
			self.$field = $field;
		}
	};
}

impl UIElement {
	// Builder macro
	builder_method!(with_position, position, Vec2);
	builder_method!(with_size, size, Vec2);
	builder_method!(with_color, color, Color);
	builder_method!(with_z_index, z_index, i32);
	builder_method!(with_visible, visible, bool);
	builder_method!(with_enabled, enabled, bool);
	builder_method!(with_border, border, Border);
	// Setter macro
	setter_method!(set_position, position, Vec2);
	setter_method!(set_size, size, Vec2);
	setter_method!(set_color, color, Color);
	setter_method!(set_z_index, z_index, i32);
	setter_method!(set_visible, visible, bool);
	setter_method!(set_enabled, enabled, bool);
	setter_method!(set_border, border, Border);
	// Builder methods
	#[inline] pub const fn with_alpha(mut self, a: u8) -> Self { self.color = self.color.with_a(a); self }
	#[inline] pub const fn with_parent(mut self, parent: usize) -> Self { self.parent = Some((parent, Vec2::new(0.,0.))); self }
	#[inline] pub const fn with_parent_off(mut self, parent: usize, offset: Vec2) -> Self { self.parent = Some((parent, offset)); self }
	// setters
	#[inline] pub const fn set_alpha(&mut self, a: u8){ self.color = self.color.with_a(a); }
	#[inline] pub const fn set_parent(&mut self, parent: usize) { self.parent = Some((parent, Vec2::new(0.,0.))); }
	#[inline] pub const fn set_parent_off(&mut self, parent: usize, offset: Vec2) { self.parent = Some((parent, offset)) }

	// Builder-style versions (return Self for chaining)
	#[inline] pub fn with_added_state(mut self, state: MutStr) -> Self { self.add_state(state); self }
	#[inline] pub fn with_added_states(mut self, states: &[MutStr]) -> Self { self.add_states(states); self }
	#[inline] pub fn with_added_frame(mut self, frame: MutStr) -> Self { self.add_frame(frame); self }
	#[inline] pub fn with_added_frames(mut self, frames: &[MutStr]) -> Self { self.add_frames(frames); self }


	// MultiStateButton states management
	#[inline] pub fn add_state(&mut self, state: MutStr) {
		if let UIElementData::MultiStateButton { states, .. } = &mut self.data {
			states.push(state);
		}
	}
	#[inline] pub fn add_states(&mut self, new_states: &[MutStr]) {
		if let UIElementData::MultiStateButton { states, .. } = &mut self.data {
			states.extend_from_slice(new_states);
		}
	}
	// Animation frames management
	#[inline] pub fn add_frame(&mut self, frame: MutStr) {
		if let UIElementData::Animation { frames, .. } = &mut self.data {
			frames.push(frame);
		}
	}
	#[inline] pub fn add_frames(&mut self, new_frames: &[MutStr]) {
		if let UIElementData::Animation { frames, .. } = &mut self.data {
			frames.extend_from_slice(new_frames);
		}
	}



	#[inline] pub const fn with_style(self, style: &ElementStyle) -> Self {
		self.with_color(style.color)
			.with_border(style.border)
			.with_text_color(style.text_color())
	}
	#[inline] pub const fn with_vertical(mut self, vertical: bool) -> Self {
		self.vertical = vertical;
		self
	}

	#[inline] pub fn with_callback<F: FnMut() + 'static>(mut self, callback: F) -> Self {
		self.event_handler = Some(Arc::new(RefCell::new(callback)));
		self
	}
			
	// Utility methods
	#[inline] pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
		let corner = self.position + self.size;
		(self.position.x, self.position.y, corner.x, corner.y)
	}
	#[inline] pub fn contains_point(&self, x: f32, y: f32) -> bool {
		if !self.visible || !self.enabled { return false; }
		let (min_x, min_y, max_x, max_y) = self.get_bounds();
		x >= min_x && x <= max_x && y >= min_y && y <= max_y
	}
	#[inline] pub const fn is_input(&self) -> bool { matches!(self.data, UIElementData::InputField { .. }) }

	#[inline] pub const fn update_hover_state(&mut self, is_hovered: bool) {
		self.hovered = is_hovered && self.enabled;
		match self.data {
			UIElementData::Button { .. } | UIElementData::InputField { .. } | UIElementData::Slider { .. } | UIElementData::MultiStateButton { .. } => {
				self.color.a = if self.hovered && self.enabled {
					Color::HOVER_ALPHA
				} else if !self.enabled {
					Color::HOVER_ALPHA / 2
				} else {
					Color::DEF_ALPHA
				};
			},
			_ => { },
		}
	}
	#[inline] pub fn get_text_mut(&mut self) -> Option<&mut String> {
		match &mut self.data {
			UIElementData::Label { text, .. } |
			UIElementData::Button { text, .. } |
			UIElementData::InputField { text, .. } |
			UIElementData::Checkbox { text, .. } => {
				Some(text.get_mut())
			},
			_ => None,
		}
	}
	#[inline] pub fn trigger_callback(&mut self) {
		if let Some(cb) = self.event_handler.clone() {
			cb.borrow_mut()();
		}
	}

	#[inline] pub fn get_element_data(&self) -> ElementData {
		match &self.data {
			UIElementData::Label { text, .. } |
			UIElementData::Button { text, .. } |
			UIElementData::InputField { text, .. } |
			UIElementData::Checkbox { text, .. } => {
				ElementData::Text(text.to_string())
			},
			UIElementData::MultiStateButton { states, current_state, .. } => {
				ElementData::Text((&states[*current_state]).to_string())
			},
			UIElementData::Animation { frames, current_frame, .. } => {
				ElementData::Text((&frames[*current_frame as usize]).to_string())
			},
			UIElementData::Slider { current_value, .. } => {
				ElementData::Number(*current_value)
			},
			_ => ElementData::None,
		}
	}
	#[inline] pub fn get_string(&self) -> Option<String> {
		self.get_element_data().text()
	}

	

	// Text-related methods
	#[inline]
	pub fn with_text(mut self, text: &str) -> Self {
		if let Some(text_field) = self.get_text_mut() { *text_field = text.to_string(); }
		self
	}
	#[inline]
	pub const fn with_text_color(mut self, color: Color) -> Self {
		match &mut self.data {
			UIElementData::Label { text_color, .. } |
			UIElementData::Button { text_color, .. } |
			UIElementData::InputField { text_color, .. } |
			UIElementData::Checkbox { text_color, .. } |
			UIElementData::MultiStateButton { text_color, .. } |
			UIElementData::Slider { text_color, .. } => *text_color = color,
			_ => {}
		}
		self
	}
	#[inline]
	pub const fn with_text_alpha(self, a: u8) -> Self {
		let color = self.get_text_color().with_a(a);
		self.with_text_color(color)
	}
	#[inline]
	pub const fn get_text_color(&self) -> Color {
		let text_color = match &self.data {
			UIElementData::Label { text_color, .. } |
			UIElementData::Button { text_color, .. } |
			UIElementData::InputField { text_color, .. } |
			UIElementData::Checkbox { text_color, .. } |
			UIElementData::MultiStateButton { text_color, .. } |
			UIElementData::Slider { text_color, .. } => Some(*text_color),
			_ => None,
		};
		if let Some(color) = text_color {
			color
		} else { self.color }
	}
	#[inline]
	pub fn with_placeholder(mut self, p: &'static str) -> Self {
		if let UIElementData::InputField { placeholder, .. } = &mut self.data {
			*placeholder = MutStr::from_str(p);
		}
		self
	}

	// MultiStateButton-related methods
	#[inline]
	pub const fn next_state(&mut self) {
		if let UIElementData::MultiStateButton { states, current_state, .. } = &mut self.data {
			*current_state = (*current_state + 1) % states.len();
		}
	}
	#[inline]
	pub fn with_states(mut self, states: Vec<MutStr>) -> Self {
		if let UIElementData::MultiStateButton { states: s, .. } = &mut self.data {
			*s = states;
		}
		self
	}
	#[inline]
	pub const fn get_current_state(&self) -> Option<usize> {
		if let UIElementData::MultiStateButton { current_state, .. } = &self.data {
			Some(*current_state)
		} else {
			None
		}
	}
	// Slider-related methods
	#[inline]
	pub const fn with_step(mut self, step: f32) -> Self {
		if let UIElementData::Slider { step: s, .. } = &mut self.data {
			*s = Some(step);
		}
		self
	}
	#[inline]
	pub const fn with_value(mut self, value: f32) -> Self {
		if let UIElementData::Slider { min_value, max_value, current_value, .. } = &mut self.data {
			*current_value = value.clamp(*min_value, *max_value);
		}
		self
	}
	#[inline]
	pub const fn set_calc_value(&mut self, norm_x: f32, norm_y: f32) {
		if let UIElementData::Slider { .. } = &mut self.data {
			if let Some(value) = self.calc_value(norm_x, norm_y) {
				self.set_value(value);
			}
		}
	}
	#[inline]
	pub const fn set_value(&mut self, value: f32) {
		if let UIElementData::Slider { current_value, .. } = &mut self.data {
			*current_value = value;
		}
	}
	#[inline]
	pub const fn calc_value(&self, norm_x: f32, _norm_y: f32) -> Option<f32> {
		let UIElementData::Slider { min_value, max_value, step, .. } = &self.data else { return None; };
		let handle_width = self.size.y * 0.8;
		let effective_width = self.size.x - handle_width;
		let click_x = (norm_x - self.position.x - handle_width / 2.0).clamp(0.0, effective_width);
		let normalized_value = click_x / effective_width;
		let value = (*min_value + normalized_value * (*max_value - *min_value)).clamp(*min_value, *max_value);

		let Some(step) = step else { return Some(value) };

		let rounded_val = (value / *step + if value >= 0.0 { 0.5 } else { -0.5 }) as i32 as f32;
		let stepped_value = (rounded_val * *step).clamp(*min_value, *max_value);
		Some(stepped_value)
	}
	#[inline]
	pub const fn get_value(&self) -> Option<f32> {
		if let UIElementData::Slider { current_value, .. } = &self.data {
			Some(*current_value)
		} else {
			None
		}
	}
	
	// Checkbox-related methods
	#[inline]
	pub const fn with_checked(mut self, checked: bool) -> Self {
		if let UIElementData::Checkbox { checked: c, .. } = &mut self.data { *c = checked; }
		self
	}
	#[inline]
	pub const fn toggle_checked(&mut self) {
		if let UIElementData::Checkbox { checked, .. } = &mut self.data { *checked = !*checked; }
	}
	#[inline]
	pub const fn is_checked(&self) -> Option<bool> {
		if let UIElementData::Checkbox { checked, .. } = &self.data { Some(*checked) } else { None }
	}

	// Animation-related methods
	#[inline]
	pub fn with_animation_frames(mut self, frames: Vec<MutStr>) -> Self {
		if let UIElementData::Animation { frames: f, .. } = &mut self.data {
			*f = frames;
		}
		self
	}
	#[inline]
	pub const fn with_animation_duration(mut self, duration: f32) -> Self {
		if let UIElementData::Animation { frame_duration, .. } = &mut self.data { *frame_duration = duration; }
		self
	}
	#[inline]
	pub const fn with_animation_looping(mut self, looping: bool) -> Self {
		if let UIElementData::Animation { looping: l, .. } = &mut self.data { *l = looping; }
		self
	}
	#[inline]
	pub const fn with_blend_delay(mut self, delay: u32) -> Self {
		if let UIElementData::Animation { blend_delay, .. } = &mut self.data { *blend_delay = Some(delay); }
		self
	}
	#[inline]
	pub const fn play_anim(&mut self) {
		if let UIElementData::Animation { playing, .. } = &mut self.data { *playing = true; }
	}
	#[inline]
	pub const fn pause_anim(&mut self) {
		if let UIElementData::Animation { playing, .. } = &mut self.data { *playing = false; }
	}
	#[inline]
	pub const fn reset_anim(&mut self) {
		if let UIElementData::Animation { current_frame, elapsed_time, .. } = &mut self.data {
			*current_frame = 0; *elapsed_time = 0.0;
		}
	}
	#[inline]
	pub const fn update_anim(&mut self, delta_time: f32) {
		let UIElementData::Animation {
			frames, current_frame, frame_duration, elapsed_time, looping, playing, ..
		} = &mut self.data else { return; };

		if !*playing || frames.is_empty() { return; }
		
		*elapsed_time += delta_time;
		while *elapsed_time >= *frame_duration {
			*elapsed_time -= *frame_duration;
			*current_frame += 1;
			
			if *current_frame < frames.len() as u32 { continue; }

			if *looping {
				*current_frame = 0;
			} else {
				*current_frame = frames.len() as u32 - 1;
				*playing = false;
				break;
			}
		}
	}
	#[inline]
	pub const fn get_packed_anim_data(&self) -> Option<[u32; 2]> {
		let UIElementData::Animation {
			frames, current_frame, frame_duration, elapsed_time, blend_delay, ..
		} = &self.data else { return None; };

		let frame_count = frames.len() as u32;
		let next_frame = if blend_delay.is_some() {
			(*current_frame + 1) % frame_count
		} else {
			*current_frame
		};
		let packed_frames = (*current_frame & 0xFFFF) | ((next_frame & 0xFFFF) << 16);
		
		// Convert to integer arithmetic by working in hundredths (like fixed-point)
		let elapsed_hundredths = (*elapsed_time * 100.0) as u32;
		let duration_hundredths = (*frame_duration * 100.0) as u32;
		
		// Calculate progress using integer division (0-100 range)
		let raw_progress = if duration_hundredths > 0 {
			(elapsed_hundredths * 100) / duration_hundredths
		} else {
			0
		};
		let delay = if blend_delay.is_some() { blend_delay.unwrap() } else { 0 };
		let packed_progress = (raw_progress & 0xFFFF) | ((delay & 0xFFFF) << 16);
		return Some([packed_frames, packed_progress]);
	}
}

// Input validation and processing
#[inline]
pub fn process_text_input(text: &mut String, c: char) -> bool {
	if text.len() >= 256 || c.is_control() { return false; }
	
	text.push(c);
	
	true
}

#[inline]
pub fn handle_backspace(text: &mut String) -> bool {
	if text.is_empty() { return false; }

	text.pop();
	
	true
}


use winit::keyboard::KeyCode as Key;

// Input handling utilities (unchanged)
#[inline]
pub const fn key_to_char(key: Key, shift: bool) -> Option<char> {
	match key {
			// Alphabet
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
			// Numbers
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
			// Symbols
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
			// Numpad keys (with NumLock on)
		Key::Numpad0 => Some('0'),
		Key::Numpad1 => Some('1'),
		Key::Numpad2 => Some('2'),
		Key::Numpad3 => Some('3'),
		Key::Numpad4 => Some('4'),
		Key::Numpad5 => Some('5'),
		Key::Numpad6 => Some('6'),
		Key::Numpad7 => Some('7'),
		Key::Numpad8 => Some('8'),
		Key::Numpad9 => Some('9'),
		Key::NumpadAdd => Some('+'),
		Key::NumpadSubtract => Some('-'),
		Key::NumpadMultiply => Some('*'),
		Key::NumpadDivide => Some('/'),
		Key::NumpadDecimal => Some('.'),
			// Fallback - undefined
		_ => None,
	}
}
