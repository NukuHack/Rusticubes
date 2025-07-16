
use crate::ui::manager;
use crate::ext::config;
use crate::game::player;

impl<'a> crate::State<'a> {

    #[inline]
    pub fn center_mouse(&self) {
        // Reset mouse to center
        let size: &winit::dpi::PhysicalSize<u32> = self.size();
        let x:f64 = (size.width as f64) / 2.0;
        let y:f64 = (size.height as f64) / 2.0;
        self.window().set_cursor_position(winit::dpi::PhysicalPosition::new(x, y))
            .expect("Set mouse cursor position");
    }

    #[inline]
    pub fn toggle_mouse_capture(&mut self) {
        if self.is_world_running  && config::get_gamestate().is_running() {
            if self.input_system.mouse_captured() {
                let player = &mut config::get_gamestate().player_mut();
                player.set_camera_mode(player::CameraMode::Smooth);
                self.input_system.set_mouse_captured(false);
                // Show cursor and release
                //self.window().set_cursor_icon(winit::window::CursorIcon::Default);
                self.window().set_cursor_visible(true);
                self.window().set_cursor_grab(winit::window::CursorGrabMode::None).unwrap();
            } else {
                if let manager::UIState::Inventory(_) = config::get_state().ui_manager.state.clone() {
                    return;
                }
                let player = &mut config::get_gamestate().player_mut();
                player.set_camera_mode(player::CameraMode::Instant);
                self.input_system.set_mouse_captured(true);
                // Hide cursor and lock to center
                //self.window().set_cursor_icon(winit::window::CursorIcon::Crosshair);
                self.window().set_cursor_visible(false);
                self.window().set_cursor_grab(winit::window::CursorGrabMode::Confined)
                    .or_else(|_| self.window().set_cursor_grab(winit::window::CursorGrabMode::Locked)).unwrap();
                self.center_mouse();
            }
        } else {
            // if the game is not running release mouse all ways
            self.input_system.set_mouse_captured(false);
            // Show cursor and release
            //self.window().set_cursor_icon(winit::window::CursorIcon::Default);
            self.window().set_cursor_visible(true);
            self.window().set_cursor_grab(winit::window::CursorGrabMode::None).unwrap();
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
CursorIcon::VerticalText  // Vertical text
CursorIcon::Alias         // Alias
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