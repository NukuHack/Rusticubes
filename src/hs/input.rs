
use winit::keyboard::ModifiersState;

pub struct InputSystem {
	pub previous_mouse: Option<winit::dpi::PhysicalPosition<f64>>,
	pub mouse_button_state: MouseButtonState,
	pub modifiers: ModifiersState,
	mouse_captured: bool,
}
impl InputSystem {
	#[inline] pub const fn default() -> Self {
		Self {
			previous_mouse: None,
			mouse_button_state: MouseButtonState::default(),
			modifiers: ModifiersState::empty(),
			mouse_captured: false,
		}
	}
}
impl InputSystem {
	#[inline] pub const fn set_mouse_captured(&mut self, is_captured:bool) {
		self.mouse_captured = is_captured;
	}

	#[inline] pub const fn mouse_captured(&self) -> bool {
		self.mouse_captured
	}
}

pub struct MouseButtonState {
	pub left: bool,
	pub right: bool,
}
impl MouseButtonState {
	pub const fn default() -> Self {
		Self {
			left : false,
			right : false,
		}
	}
}