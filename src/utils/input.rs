
use std::collections::HashSet;
use winit::event::MouseButton;
use winit::keyboard::ModifiersState;
use winit::dpi::PhysicalPosition;
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

	#[inline] pub const fn default() -> Self {
		Self {
			previous_mouse: PhysicalPosition::new(0.0, 0.0),
			mouse_button_state: MouseButtonState::default(),
			modifiers: ModifiersState::empty(),
			drag_state: DragState::NotDragging,
			mouse_captured: false,
		}
	}

	#[inline] pub fn clear(&mut self) {
		let is_mouse_captured: bool = self.is_mouse_captured();
		*self = Self::default();
		self.set_mouse_captured(is_mouse_captured);
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
	pub const fn from(button: MouseButton) -> Self {
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
	pub const fn default() -> Self {
		Self {
			left: false,
			right: false,
			middle: false,
			back: false,
			forward: false,
		}
	}
}
