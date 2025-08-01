
use crate::ext::color::{Color, Border};
use std::{cell::RefCell, sync::Arc };
use crate::ext::config::ElementStyle;

type Callback = Arc<RefCell<dyn FnMut() + 'static>>;

#[derive(Clone)]
pub enum UIElementData {
	Panel { on_click: Option<Callback> },
	Label { text: &'static str, text_color: Color, on_click: Option<Callback> },
	Button { text: &'static str, text_color: Color, on_click: Option<Callback> },
	MultiStateButton { states: Vec<&'static str>, text_color: Color, current_state: usize, on_click: Option<Callback> },
	InputField { text: &'static str, text_color: Color, placeholder: Option<&'static str>, on_click: Option<Callback> },
	Checkbox { label: Option<&'static str>, text_color: Color, checked: bool, on_click: Option<Callback> },
	Image { path: &'static str },
	Animation {
		frames: Vec<&'static str>, current_frame: u32, frame_duration: f32, elapsed_time: f32,
		looping: bool, playing: bool, blend_delay: Option<u32>, on_click: Option<Callback>
	},
	Slider {
		min_value: f32, slider_color: Color, max_value: f32, current_value: f32,
		step: Option<f32>, on_change: Option<Callback>,
	},
}
impl UIElementData {
	#[inline] pub const fn default() -> Self { UIElementData::Panel{ on_click: None } }
}


#[derive(Clone)]
pub struct UIElement {
	pub id: usize,
	pub data: UIElementData,
	pub position: (f32, f32),
	pub size: (f32, f32),
	pub color: Color,
	pub hovered: bool,
	pub z_index: i32,
	pub visible: bool,
	pub border: Border,
	pub enabled: bool,
	pub vertical: bool,
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
			position: (0.0, 0.0),
			size: (0.0, 0.0),
			color: Color::DEF_COLOR,
			hovered: false,
			z_index: 0,
			visible: true,
			border: Border::NONE,
			enabled: true,
			vertical: false,
		}
	}
	#[inline]
	pub const fn panel(id: usize) -> Self { Self::new(id, UIElementData::default() ) }
	#[inline]
	pub const fn label(id: usize, text: &'static str) -> Self {
		Self::new(id, UIElementData::Label { text, text_color: Color::DEF_COLOR, on_click: None })
	}
	#[inline]
	pub const fn button(id: usize, text: &'static str) -> Self {
		Self::new(id, UIElementData::Button { text, text_color: Color::DEF_COLOR, on_click: None })
	}
	#[inline]
	pub const fn input(id: usize) -> Self {
		Self::new(id, UIElementData::InputField { text: "", text_color: Color::DEF_COLOR, placeholder: None, on_click: None })
	}
	#[inline]
	pub const fn checkbox(id: usize, label: Option<&'static str>) -> Self {
		Self::new(id, UIElementData::Checkbox { label, text_color: Color::DEF_COLOR, checked: false, on_click: None })
	}
	#[inline]
	pub const fn image(id: usize, path: &'static str) -> Self {
		Self::new(id, UIElementData::Image { path })
	}
	#[inline]
	pub const fn animation(id: usize, frames: Vec<&'static str>) -> Self {
		Self::new(id, UIElementData::Animation {
			frames, current_frame: 0, frame_duration: 1.0, elapsed_time: 0.0,
			looping: true, playing: true, blend_delay: Some(20), on_click: None
		})
	}
	#[inline]
	pub const fn multi_state_button(id: usize, states: Vec<&'static str>) -> Self {
		Self::new(id, UIElementData::MultiStateButton {
			states, text_color: Color::DEF_COLOR,
			current_state: 0,on_click: None,
		})
	}
	#[inline]
	pub const fn slider(id: usize, min_value: f32, max_value: f32) -> Self {
		Self::new(id, UIElementData::Slider {
			min_value, max_value, //vertical: false,
			slider_color: Color::DEF_COLOR,
			current_value: min_value,
			step: None,on_change: None, 
		})
	}
	
	
	// Builder methods
	#[inline] pub const fn with_position(mut self, x: f32, y: f32) -> Self { self.position = (x, y); self }
	#[inline] pub const fn with_size(mut self, width: f32, height: f32) -> Self { self.size = (width, height); self }
	#[inline] pub const fn with_color(mut self, color: Color) -> Self { self.color = color; self }
	#[inline] pub const fn with_alpha(mut self, a: u8) -> Self { self.color = self.color.with_a(a); self }
	#[inline] pub const fn with_z_index(mut self, z_index: i32) -> Self { self.z_index = z_index; self }
	#[inline] pub const fn with_visible(mut self, visible: bool) -> Self { self.visible = visible; self }
	#[inline] pub const fn with_enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
	#[inline] pub const fn with_border(mut self, border: Border) -> Self { self.border = border; self}

	#[inline]
	pub const fn with_style(self, style: &ElementStyle) -> Self {
		self.with_color(style.color)
			.with_border(style.border)
			.with_text_color(style.text_color())
	}
	#[inline]
	pub const fn with_vertical(mut self, vertical: bool) -> Self {
		self.vertical = vertical;
		self
	}
	#[inline]
	pub fn with_data(self, data: UIElementData) -> Self {
		let mut result = self;
		result.data = data;
		result
	}
	#[inline]
	pub const fn with_id(mut self, id: usize) -> Self {
		self.id = id;
		self
	}

	#[inline]
	pub fn with_callback<F: FnMut() + 'static>(mut self, callback: F) -> Self {
		match &mut self.data {
			UIElementData::Button { on_click, .. } => {
				*on_click = Some(Arc::new(RefCell::new(callback)));
			}
			UIElementData::Checkbox { on_click, .. } => {
				*on_click = Some(Arc::new(RefCell::new(callback)));
			}
			UIElementData::MultiStateButton { on_click, .. } => {
				*on_click = Some(Arc::new(RefCell::new(callback)));
			}
			UIElementData::Slider { on_change, .. } => {
				*on_change = Some(Arc::new(RefCell::new(callback)));
			}
			_ => {}
		}
		self
	}
			
	// Utility methods
	#[inline] pub const fn get_bounds(&self) -> (f32, f32, f32, f32) {
		let (x, y) = self.position;
		let (w, h) = self.size;
		(x, y, x + w, y + h)
	}
	#[inline] pub const fn contains_point(&self, x: f32, y: f32) -> bool {
		if !self.visible || !self.enabled { return false; }
		let (min_x, min_y, max_x, max_y) = self.get_bounds();
		x >= min_x && x <= max_x && y >= min_y && y <= max_y
	}
	#[inline] pub const fn is_input(&self) -> bool { matches!(self.data, UIElementData::InputField { .. }) }
	#[inline] pub const fn update_hover_state(&mut self, is_hovered: bool) {
		self.hovered = is_hovered && self.enabled;
		match self.data {
			UIElementData::Button{ .. } | UIElementData::InputField{ .. } | UIElementData::Slider{ .. } | UIElementData::MultiStateButton{ .. } => {

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
	#[inline]
	pub const fn get_text_mut(&mut self) -> Option<&mut &'static str> {
		match &mut self.data {
			UIElementData::Label { text, .. } |
			UIElementData::Button { text, .. } |
			UIElementData::InputField { text, .. } => Some(text),
			UIElementData::Checkbox { label, .. } => label.as_mut(),
			_ => None,
		}
	}
	#[inline]
	pub fn trigger_callback(&mut self) {
		let callback = match &mut self.data {
			UIElementData::Button { on_click, .. } => on_click.clone(),
			UIElementData::Checkbox { on_click, .. } => on_click.clone(),
			UIElementData::MultiStateButton { on_click, .. } => on_click.clone(),
			UIElementData::Slider { on_change, .. } => on_change.clone(),
			_ => None,
		};
		if let Some(cb) = callback {
			cb.borrow_mut()();
		}
	}


	#[inline]
	pub fn get_element_data(&self) -> ElementData {
		match &self.data {
			UIElementData::Label { text, .. } |
			UIElementData::Button { text, .. } |
			UIElementData::InputField { text, .. } => {
				ElementData::Text(text.trim())
			},
			UIElementData::Checkbox { label, .. } => {
				label.map(ElementData::Text).unwrap_or(ElementData::None)
			},
			UIElementData::MultiStateButton { states, current_state, .. } => {
				ElementData::Text(&states[*current_state])
			},
			UIElementData::Animation { frames, current_frame, .. } => {
				ElementData::Text(&frames[*current_frame as usize])
			},
			UIElementData::Slider { current_value, .. } => {
				ElementData::Number(*current_value)
			},
			_ => ElementData::None,
		}
	}
	#[inline]
	pub fn get_str(&self) -> Option<&'static str> {
		self.get_element_data().text()
	}



	// setters
	#[inline] pub const fn set_position(&mut self, x: f32, y: f32) { self.position = (x, y); }
	#[inline] pub const fn set_size(&mut self, width: f32, height: f32) { self.size = (width, height); }
	#[inline] pub const fn set_color(&mut self, color: Color) { self.color = color; }
	#[inline] pub const fn set_alpha(&mut self, a: u8){ self.color = self.color.with_a(a); }
	#[inline] pub const fn set_z_index(&mut self, z_index: i32){ self.z_index = z_index; }
	#[inline] pub const fn set_visible(&mut self, visible: bool) { self.visible = visible; }
	#[inline] pub const fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
	#[inline] pub const fn set_border(&mut self, border: Border) { self.border = border; }
	
}
#[derive(Debug)]
pub enum ElementData {
	Text(&'static str),
	Number(f32),
	None,
}
impl ElementData {
	pub const fn text(&self) -> Option<&'static str> {
		match self {
			ElementData::Text(s) if !s.is_empty() => Some(s),
			_ => None,
		}
	}
	pub const fn num(&self) -> Option<f32> {
		match self {
			ElementData::Number(s) => Some(*s),
			_ => None,
		}
	}
}


impl UIElement {
	// Text-related methods
	#[inline]
	pub const fn with_text(mut self, text: &'static str) -> Self {
		if let Some(text_field) = self.get_text_mut() { *text_field = text; }
		self
	}
	#[inline]
	pub const fn with_text_color(mut self, color: Color) -> Self {
		match &mut self.data {
			UIElementData::Label { text_color, .. } |
			UIElementData::Button { text_color, .. } |
			UIElementData::InputField { text_color, .. } |
			UIElementData::Checkbox { text_color, .. } |
			UIElementData::MultiStateButton { text_color, .. } => *text_color = color,
			UIElementData::Slider { slider_color, .. } => *slider_color = color,
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
			UIElementData::MultiStateButton { text_color, .. } => Some(*text_color),
			UIElementData::Slider { slider_color, .. } => Some(*slider_color),
			_ => None,
		};
		if let Some(color) = text_color {
			color
		} else { self.color }
	}
	#[inline]
	pub const fn with_placeholder(mut self, placeholder: Option<&'static str>) -> Self {
		if let UIElementData::InputField { placeholder: p, .. } = &mut self.data {
			*p = placeholder;
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
	pub fn with_states(mut self, states: Vec<&'static str>) -> Self {
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
	/*
	#[inline]
	pub const fn with_vertical(mut self, vertical: bool) -> Self {
		if let UIElementData::Slider { vertical: v, .. } = &mut self.data {
			*v = vertical;
		}
		self
	}*/
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
		if let UIElementData::Slider { min_value, max_value, step, .. } = &self.data {
			// Calculate clicked position relative to slider
			let (x, y) = self.position;
			let (w, h) = self.size;
			
			// Get click position relative to slider track
			let track_height = h * 0.5;
			let _track_y = y + (h - track_height) / 2.0;
			
			{
				let handle_width = h * 0.8;
				let effective_width = w - handle_width;
				let click_x = if norm_x - x - handle_width / 2.0 < 0.0 {
					0.0
				} else if norm_x - x - handle_width / 2.0 > effective_width {
					effective_width
				} else {
					norm_x - x - handle_width / 2.0
				};
				
				// Calculate normalized value (0-1)
				let normalized_value = click_x / effective_width;
				
				// Convert to actual value range
				let value = *min_value + normalized_value * (*max_value - *min_value);
				
				let clamped_value = if value < *min_value {
					*min_value
				} else if value > *max_value {
					*max_value
				} else {
					value
				};

				if let Some(step) = step {
					// Round to the nearest step using integer casting
					let val = clamped_value / *step;
					let rounded_val = if val >= 0.0 {
						(val + 0.5) as i32 as f32
					} else {
						(val - 0.5) as i32 as f32
					};
					let stepped_value = rounded_val * *step;
					
					// Ensure we're still within bounds after rounding
					if stepped_value < *min_value {
						return Some(*min_value);
					} else if stepped_value > *max_value {
						return Some(*max_value);
					} else {
						return Some(stepped_value);
					}
				}

				return Some(clamped_value);
			}
		}
		None
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
	pub fn with_animation_frames(mut self, frames: Vec<&'static str>) -> Self {
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
	pub const fn with_smooth_transition(mut self, smooth: bool) -> Self {
		if let UIElementData::Animation { blend_delay, .. } = &mut self.data {
			if smooth {
				if blend_delay.is_none() {
					*blend_delay = Some(0);
				}
			} else {
				*blend_delay = None;
			}
		}
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
		if let UIElementData::Animation {
			frames, current_frame, frame_duration, elapsed_time, looping, playing, ..
		} = &mut self.data {
			if !*playing || frames.is_empty() { return; }
			
			*elapsed_time += delta_time;
			while *elapsed_time >= *frame_duration {
				*elapsed_time -= *frame_duration;
				*current_frame += 1;
				
				if *current_frame >= frames.len() as u32 {
					if *looping {
						*current_frame = 0;
					} else {
						*current_frame = frames.len() as u32 - 1;
						*playing = false;
						break;
					}
				}
			}
		}
	}
	#[inline]
	pub const fn get_packed_anim_data(&self) -> Option<[u32; 2]> {
		if let UIElementData::Animation {
			frames,
			current_frame,
			frame_duration,
			elapsed_time,
			blend_delay,
			..
		} = &self.data
		{
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
			let delay = if blend_delay.is_some() {blend_delay.unwrap()} else {0};
			let packed_progress = (raw_progress & 0xFFFF) | ((delay & 0xFFFF) << 16);
			return Some([packed_frames, packed_progress]);
		}
		None
	}

}

// Input validation and processing
#[inline]
pub fn process_text_input(text: &mut &'static str, c: char) -> bool {
	if text.len() >= 256 || c.is_control() {
		return false;
	}
	
	// Create a new string with the added character
	let mut new_string = String::with_capacity(text.len() + 1);
	new_string.push_str(text);
	new_string.push(c);
	
	// Convert to &'static str and update the reference
	let boxed_str = new_string.into_boxed_str();
	let leaked = Box::leak(boxed_str);
	*text = leaked;
	
	true
}

#[inline]
pub fn handle_backspace(text: &mut &'static str) -> bool {
	if !text.is_empty() {
		// Create a new string without the last character
		let mut new_string = String::with_capacity(text.len() - 1);
		new_string.push_str(&text[..text.len()-1]);
		
		// Convert to &'static str and update the reference
		let boxed_str = new_string.into_boxed_str();
		let leaked = Box::leak(boxed_str);
		*text = leaked;
		
		true
	} else {
		false
	}
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
