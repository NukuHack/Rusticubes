use std::collections::HashSet;
use winit::event::MouseButton;
use winit::keyboard::ModifiersState;
use winit::dpi::PhysicalPosition;
use winit::keyboard::KeyCode as Key;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct DragSample {
	pub position: PhysicalPosition<f32>,
	pub timestamp: f32, // seconds since drag start
}
// Custom PartialEq that only compares position
impl PartialEq for DragSample {
	fn eq(&self, other: &Self) -> bool {
		self.position == other.position
	}
}
use std::hash::{Hash, Hasher};
impl Hash for DragSample {
	fn hash<H: Hasher>(&self, state: &mut H) {
		// Hash the x and y coordinates individually
		self.position.x.to_bits().hash(state);
		self.position.y.to_bits().hash(state);
	}
}
// Also implement Eq since we're only using position which should be Eq
impl Eq for DragSample {}

#[derive(Debug)]
pub struct InputSystem {
	previous_mouse: PhysicalPosition<f64>,
	mouse_button_state: MouseButtonState,
	modifiers: ModifiersState,
	keyboard: Keyboard,
	drag_state: DragState,
	mouse_captured: bool,
}

macro_rules! setter_method {
	($name:ident, $field:ident: $type:ty) => {
		#[inline] pub const fn $name(&mut self, $field: $type) {
			self.$field = $field;
		}
	};
}

macro_rules! getter_method {
	($field:ident: $type:ty) => {
		#[inline] pub const fn $field(&self) -> &$type {
			&self.$field
		}
	};
}

impl InputSystem {
	setter_method!(set_modifiers, modifiers: ModifiersState);
	setter_method!(set_mouse_captured, mouse_captured: bool);

	getter_method!(previous_mouse: PhysicalPosition<f64>);
	getter_method!(mouse_button_state: MouseButtonState);
	getter_method!(modifiers: ModifiersState);
	getter_method!(drag_state: DragState);
	getter_method!(keyboard: Keyboard);

	#[inline] pub const fn default() -> Self {
		Self {
			previous_mouse: PhysicalPosition::new(0.0, 0.0),
			mouse_button_state: MouseButtonState::default(),
			modifiers: ModifiersState::empty(),
			keyboard: Keyboard::default(),
			drag_state: DragState::NotDragging,
			mouse_captured: false,
		}
	}

	#[inline] pub fn clear(&mut self) {
		let is_mouse_captured: bool = self.is_mouse_captured();
		*self = Self::default();
		self.set_mouse_captured(is_mouse_captured);
	}

	pub fn handle_key_input(&mut self, key: Key, is_pressed: bool) {
		match key {
			// Movement keys (keeping original)
			Key::KeyW => self.keyboard.w = is_pressed,
			Key::KeyA => self.keyboard.a = is_pressed,
			Key::KeyS => self.keyboard.s = is_pressed,
			Key::KeyD => self.keyboard.d = is_pressed,
			
			// All other English letters
			Key::KeyQ => self.keyboard.q = is_pressed,
			Key::KeyE => self.keyboard.e = is_pressed,
			Key::KeyR => self.keyboard.r = is_pressed,
			Key::KeyT => self.keyboard.t = is_pressed,
			Key::KeyY => self.keyboard.y = is_pressed,
			Key::KeyU => self.keyboard.u = is_pressed,
			Key::KeyI => self.keyboard.i = is_pressed,
			Key::KeyO => self.keyboard.o = is_pressed,
			Key::KeyP => self.keyboard.p = is_pressed,
			Key::KeyF => self.keyboard.f = is_pressed,
			Key::KeyG => self.keyboard.g = is_pressed,
			Key::KeyH => self.keyboard.h = is_pressed,
			Key::KeyJ => self.keyboard.j = is_pressed,
			Key::KeyK => self.keyboard.k = is_pressed,
			Key::KeyL => self.keyboard.l = is_pressed,
			Key::KeyZ => self.keyboard.z = is_pressed,
			Key::KeyX => self.keyboard.x = is_pressed,
			Key::KeyC => self.keyboard.c = is_pressed,
			Key::KeyV => self.keyboard.v = is_pressed,
			Key::KeyB => self.keyboard.b = is_pressed,
			Key::KeyN => self.keyboard.n = is_pressed,
			Key::KeyM => self.keyboard.m = is_pressed,
			
			// Space
			Key::Space => self.keyboard.space = is_pressed,
			
			// Modifier keys
			Key::ShiftLeft => self.keyboard.shift_left = is_pressed,
			Key::ShiftRight => self.keyboard.shift_right = is_pressed,
			Key::ControlLeft => self.keyboard.ctrl_left = is_pressed,
			Key::ControlRight => self.keyboard.ctrl_right = is_pressed,
			Key::AltLeft => self.keyboard.alt_left = is_pressed,
			Key::AltRight => self.keyboard.alt_right = is_pressed,
			Key::SuperLeft => self.keyboard.super_left = is_pressed,
			Key::SuperRight => self.keyboard.super_right = is_pressed,
			
			_ => {},
		}
	}
	
	#[inline] pub const fn reset_keyboard(&mut self) {
		self.keyboard = Keyboard::default();
	}

	#[inline] pub const fn is_dragging(&self) -> bool {
		!matches!(self.drag_state, DragState::NotDragging)
	}
	#[inline] pub const fn is_mouse_captured(&self) -> bool {
		self.mouse_captured
	}

	#[inline] pub fn is_any_mouse_button_pressed(&self) -> bool {
		self.mouse_button_state.left ||
		self.mouse_button_state.right ||
		self.mouse_button_state.middle ||
		self.mouse_button_state.back ||
		self.mouse_button_state.forward
	}

	pub fn handle_mouse_event(&mut self, button: MouseButton, pressed: bool, position: PhysicalPosition<f64>) {
		match button {
			MouseButton::Left => self.mouse_button_state.left = pressed,
			MouseButton::Right => self.mouse_button_state.right = pressed,
			MouseButton::Middle => self.mouse_button_state.middle = pressed,
			MouseButton::Back => self.mouse_button_state.back = pressed,
			MouseButton::Forward => self.mouse_button_state.forward = pressed,
			MouseButton::Other(_) => {},
		}

		if pressed {
			if !self.is_dragging() && self.is_any_mouse_button_pressed() {
				self.start_drag(ClickMode::from(button), position);
			}
		} else {
			if self.is_dragging() && !self.is_any_mouse_button_pressed() {
				self.drag_state = DragState::NotDragging;
			}
		}
	}

	#[inline] pub fn handle_mouse_move(&mut self, position: PhysicalPosition<f64>) {
		if self.is_dragging() {
			self.update_drag(position);
		}
		self.previous_mouse = position;
	}

	fn start_drag(&mut self, button: ClickMode, start_pos: PhysicalPosition<f64>) {
		let now = Instant::now();
		let mut samples = Vec::new();
		
		// Add the starting point
		samples.push(DragSample {
			position: PhysicalPosition::new(start_pos.x as f32, start_pos.y as f32),
			timestamp: 0.0,
		});

		self.drag_state = DragState::Dragging {
			button,
			start_pos,
			current_pos: start_pos,
			start_modifiers: self.modifiers,
			current_modifiers: self.modifiers,
			samples,
			last_sample_pos: start_pos,
			start_time: now,
		};
	}

	fn update_drag(&mut self, pos: PhysicalPosition<f64>) {
		if let DragState::Dragging { 
			ref mut current_pos, 
			ref mut samples, 
			ref mut last_sample_pos,
			start_time, .. } = self.drag_state {
			*current_pos = pos;
			
			// Calculate distance from last sampled point
			let dx = pos.x - last_sample_pos.x;
			let dy = pos.y - last_sample_pos.y;
			let distance_squared = dx * dx + dy * dy;
			
			const SAMPLE_DISTANCE_SQUARED: f64 = 0.1; // if movement is bigger than one tenth of a pixel save it 
			
			if distance_squared >= SAMPLE_DISTANCE_SQUARED {
				let elapsed = start_time.elapsed().as_secs_f32();
				
				samples.push(DragSample {
					position: PhysicalPosition::new(pos.x as f32, pos.y as f32),
					timestamp: elapsed,
				});
				
				*last_sample_pos = pos;
			}
		}
	}

	#[inline] pub fn get_drag_delta(&self) -> (f64, f64) {
		let DragState::Dragging { start_pos, current_pos, .. } = self.drag_state else { 
			return (0.0, 0.0); 
		};
		
		(current_pos.x - start_pos.x, current_pos.y - start_pos.y)
	}

	/// Get the full drag path samples. Returns None if not currently dragging.
	pub fn get_drag_samples(&self) -> Option<&Vec<DragSample>> {
		if let DragState::Dragging { ref samples, .. } = self.drag_state {
			Some(samples)
		} else {
			None
		}
	}

	#[inline] pub fn get_drag_unique_samples(&self) -> Option<Vec<&DragSample>> {
		if let DragState::Dragging { ref samples, .. } = self.drag_state {
			let mut seen = HashSet::new();
			let mut unique_samples = Vec::new();
			
			for sample in samples {
				if seen.insert(sample) {
					unique_samples.push(sample);
				}
			}
			
			Some(unique_samples)
		} else {
			None
		}
	}

	#[inline] pub fn compare(&self, other: &ModifiersState) -> ModifiersState {
		ModifiersState::from_bits(self.modifiers.bits() ^ other.bits()).unwrap_or_default()
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragState {
	NotDragging,
	Dragging {
		button: ClickMode,
		start_pos: PhysicalPosition<f64>,
		current_pos: PhysicalPosition<f64>,
		start_modifiers: ModifiersState,
		current_modifiers: ModifiersState,
		samples: Vec<DragSample>,
		start_time: Instant,
		last_sample_pos: PhysicalPosition<f64>, // Track last sampled position for distance calculation
	},
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClickMode {
	Left,
	Right,
	Middle,
}

impl ClickMode {
	#[inline] pub const fn from(button: MouseButton) -> Self {
		match button {
			MouseButton::Left => ClickMode::Left,
			MouseButton::Right => ClickMode::Right,
			MouseButton::Middle => ClickMode::Middle,
			MouseButton::Back => todo!(),
			MouseButton::Forward => todo!(),
			MouseButton::Other(_) => todo!(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseButtonState {
	pub left: bool,
	pub right: bool,
	pub middle: bool,
	pub back: bool,
	pub forward: bool,
}

impl MouseButtonState {
	#[inline] pub const fn default() -> Self {
		Self {
			left: false,
			right: false,
			middle: false,
			back: false,
			forward: false,
		}
	}
}



/// Input mapping configuration for flexible key binding
#[derive(Debug, Clone)]
pub struct InputMapping {
	pub forward: fn(&Keyboard) -> bool,
	pub backward: fn(&Keyboard) -> bool,
	pub left: fn(&Keyboard) -> bool,
	pub right: fn(&Keyboard) -> bool,
	pub up: fn(&Keyboard) -> bool,
	pub down: fn(&Keyboard) -> bool,
	pub run: fn(&Keyboard) -> bool,
}
impl InputMapping {
	pub const fn default() -> Self {
		Self {
			forward: |kb| kb.w,
			backward: |kb| kb.s,
			left: |kb| kb.a,
			right: |kb| kb.d,
			up: |kb| kb.space,
			down: |kb| kb.is_ctrl(),
			run: |kb| kb.is_shift(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyboard {
	// All English letters
	pub w: bool,
	pub a: bool,
	pub s: bool,
	pub d: bool,
	pub q: bool,
	pub e: bool,
	pub r: bool,
	pub t: bool,
	pub y: bool,
	pub u: bool,
	pub i: bool,
	pub o: bool,
	pub p: bool,
	pub f: bool,
	pub g: bool,
	pub h: bool,
	pub j: bool,
	pub k: bool,
	pub l: bool,
	pub z: bool,
	pub x: bool,
	pub c: bool,
	pub v: bool,
	pub b: bool,
	pub n: bool,
	pub m: bool,
	
	// Space
	pub space: bool,
	
	// Modifier keys
	pub shift_left: bool, pub shift_right: bool,
	pub ctrl_left: bool, pub ctrl_right: bool,
	pub alt_left: bool, pub alt_right: bool,
	pub super_left: bool, pub super_right: bool,  // Windows key / Cmd key
}

impl Keyboard {
	#[inline] pub const fn default() -> Self {
		Self {
			// All letters
			w: false,
			a: false,
			s: false,
			d: false,
			q: false,
			e: false,
			r: false,
			t: false,
			y: false,
			u: false,
			i: false,
			o: false,
			p: false,
			f: false,
			g: false,
			h: false,
			j: false,
			k: false,
			l: false,
			z: false,
			x: false,
			c: false,
			v: false,
			b: false,
			n: false,
			m: false,
			
			// Space
			space: false,
			
			// Modifiers
			shift_left: false, shift_right: false,
			ctrl_left: false, ctrl_right: false,
			alt_left: false, alt_right: false,
			super_left: false, super_right: false,
		}
	}
	
	// Helper methods for checking modifier combinations
	#[inline] pub const fn is_shift(&self) -> bool {
		self.shift_left || self.shift_right
	}
	#[inline] pub const fn is_ctrl(&self) -> bool {
		self.ctrl_left || self.ctrl_right
	}
	#[inline] pub const fn is_alt(&self) -> bool {
		self.alt_left || self.alt_right
	}
	#[inline] pub const fn is_super(&self) -> bool {
		self.super_left || self.super_right
	}
}
