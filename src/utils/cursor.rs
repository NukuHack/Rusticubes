
use crate::ui::manager;
use crate::ext::ptr;
use crate::game::player;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use winit::window::CursorIcon;
use winit::window::CursorGrabMode;

impl<'a> crate::State<'a> {
	#[inline] pub fn center_mouse(&mut self) {
		if !self.window().has_focus() { return } // Don't try to center if window not focused
		let size: &PhysicalSize<u32> = self.size();
		let (center_x, center_y) = (size.width as f64 / 2.0, size.height as f64 / 2.0);

		let center = PhysicalPosition::new(center_x, center_y);
		self.input_system.set_previous_mouse(center);
	
		if let Err(e) = self.window().set_cursor_position(center) {
			println!("error: {:?}", e);
		}
	}

	#[inline]
	pub fn toggle_mouse_capture(&mut self) {
		if !self.window().has_focus() { return } // Don't try to center if window not focused
		if self.is_world_running && !self.input_system.is_mouse_captured() {
			// if not in game ofc do not process
			if !ptr::get_gamestate().is_running() || !matches!(ptr::get_state().ui_manager.state, manager::UIState::InGame)
				{ return }
			self.input_system.set_mouse_captured(true);
			self.window().set_cursor_visible(false);
			self.window().set_cursor_grab(CursorGrabMode::Confined)
				.or_else(|_| self.window().set_cursor_grab(CursorGrabMode::Locked)).unwrap();
			self.center_mouse();
		} else {
			// if the game is not running release mouse all ways
			let player = &mut ptr::get_gamestate().player_mut();
			player.set_camera_mode(player::CameraMode::Instant);
			self.input_system.set_mouse_captured(false);
			// Show cursor and release
			self.window().set_cursor_icon(CursorIcon::Default);
			self.center_mouse(); // to set it correctly 
			self.window().set_cursor_visible(true);
			self.window().set_cursor_grab(CursorGrabMode::None).unwrap();
		}
	}
}




/*

CursorIcon::Default       // Default cursor
CursorIcon::Crosshair     // Crosshair
CursorIcon::Hand          // Hand
CursorIcon::Arrow         // Arrow
CursorIcon::Move          // Move
CursorIcon::Text          // Text (I-beam)
CursorIcon::Wait          // Wait (hourglass)
CursorIcon::Help          // Help (arrow with question mark)
CursorIcon::Progress      // Progress (arrow with hourglass)
CursorIcon::NotAllowed    // Not allowed
CursorIcon::ContextMenu   // Context menu
CursorIcon::Cell          // Cell
CursorIcon::Copy          // Copy
CursorIcon::NoDrop        // No drop
CursorIcon::Grab          // Grab
CursorIcon::Grabbing      // Grabbing
CursorIcon::AllScroll     // All scroll
CursorIcon::ZoomIn        // Zoom in
CursorIcon::ZoomOut       // Zoom out
CursorIcon::EResize       // East resize
CursorIcon::NResize       // North resize


*/
