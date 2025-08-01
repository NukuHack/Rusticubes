
use crate::ext::ptr;
use crate::block::extra;
use crate::ui::{manager, inventory};
use std::iter::Iterator;
use winit::{
	event::{ElementState, MouseButton, WindowEvent, MouseScrollDelta},
	keyboard::KeyCode as Key,
};

impl<'a> crate::State<'a> {

	#[inline]
	pub fn handle_events(&mut self,event: &WindowEvent) -> bool{

		// should rework this like this :
			// send the entire event to "window event handler"
		// if not processed 
			// send the entire event to "UI event handler"
		// if not processed again
			// send the entire event to "world event handler"

		// if not processed then basically it's an event what should not be processed entirely so probably log it or idk 

		match event {
			WindowEvent::CloseRequested => {ptr::close_app(); true},
			WindowEvent::Resized(physical_size) => self.resize(*physical_size),
			WindowEvent::RedrawRequested => {
				self.window().request_redraw();
				self.update();
				match self.render() {
					Ok(_) => true,
					Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
						self.resize(*self.size())
					},
					Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
						println!("Surface error");
						ptr::close_app(); true
					}
					Err(wgpu::SurfaceError::Timeout) => {
						println!("Surface timeout");
						true
					},
				}
			},
			WindowEvent::Focused(focused) => {
				if !focused{
					self.input_system.clear();
					if self.is_world_running {
						ptr::get_gamestate().player_mut().controller().reset_keyboard(); // Temporary workaround
					}
					self.ui_manager.clear_focused_element();
				} else {
					if self.is_world_running && ptr::get_gamestate().is_running() {
						// idk some stuff on focus getting ?
					}
				}
				true
			},
			WindowEvent::ModifiersChanged(modifiers) => {
				self.input_system.modifiers = modifiers.state();
				true
			},
			WindowEvent::KeyboardInput { .. } => {
				self.handle_key_input(event);
				true
			},
			WindowEvent::MouseInput { .. } |
			WindowEvent::CursorMoved { .. } |
			WindowEvent::MouseWheel { .. } => {
				self.handle_mouse_input(event);
				true
			},
			_ => false,
		}
	}
	#[inline]
	pub fn handle_key_input(&mut self, event: &WindowEvent) -> bool {
		match event {
			WindowEvent::KeyboardInput {
				event: winit::event::KeyEvent {
					physical_key,
					state // ElementState::Released or ElementState::Pressed
					, .. },..
			} => {
				let key:Key = match physical_key {
					winit::keyboard::PhysicalKey::Code(code) => *code,
					_ => {
						println!("You called a function that can only be called with a keyboard input ... without a keyboard input ... FF"); 
						return false;
					},
				};
				// let is_pressed: bool = *state == ElementState::Pressed;
				let is_pressed:bool = match state {
					ElementState::Pressed => true,
					_ => false,
				};
				// Handle UI input first if there's a focused element
				if self.ui_manager.visibility == true && self.ui_manager.focused_is_some() { // if focused you can't press Esc, have to handle them in a custom way
					if self.is_world_running {
						ptr::get_gamestate().player_mut().controller().reset_keyboard(); // Temporary workaround
					}
					if is_pressed {
						// Handle keys for UI
						self.ui_manager.handle_key_input(key, self.input_system.modifiers.shift_key());
						return true;
					}
					return true;
				}
				// Handle game controls if no UI element is focused
				// `key` is of type `KeyCode` (e.g., KeyCode::W)
				// `state` is of type `ElementState` (Pressed or Released)
				if self.is_world_running && ptr::get_gamestate().is_running() {
					ptr::get_gamestate().player_mut().controller().process_keyboard(&key, is_pressed);
					match key {
						Key::KeyF => {
							if is_pressed {
								extra::place_looked_block();
								return true
							}
						},
						Key::KeyR => {
							if is_pressed {
								extra::remove_targeted_block();
								return true
							}
						},
						Key::KeyL => {
							if is_pressed {
								extra::add_full_chunk();
								return true
							}
						},
						Key::KeyI => {
							if is_pressed {
								let state = ptr::get_state();
								match state.ui_manager.state.clone() {
									manager::UIState::Inventory(_) => {
										state.ui_manager.state = manager::UIState::InGame;
										self.toggle_mouse_capture();
									},
									manager::UIState::InGame => {
										state.ui_manager.state = manager::UIState::Inventory(inventory::InventoryUIState::default());
										if self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
									}
									_ => return false,
								}
								state.ui_manager.setup_ui();
								return true
							}
						},
						Key::KeyJ => {
							if is_pressed {
								let state = ptr::get_state();
								match state.ui_manager.state.clone() {
									manager::UIState::InGame => {
										state.ui_manager.state = manager::UIState::Inventory(inventory::InventoryUIState::str().b());
										if self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
									}
									_ => return false,
								}
								state.ui_manager.setup_ui();
								return true
							}
						},
						Key::KeyK => {
							if is_pressed {
								let state = ptr::get_state();
								match state.ui_manager.state.clone() {
									manager::UIState::InGame => {
										state.ui_manager.state = manager::UIState::Inventory(inventory::InventoryUIState::craft().b());
										if self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
									}
									_ => return false,
								}
								state.ui_manager.setup_ui();
								return true
							}
						},
						_ => { },
					};
				}
				match key {
					Key::AltLeft | Key::AltRight => {
						if is_pressed {
							self.toggle_mouse_capture();
						}
						self.center_mouse();
						true
					},
					Key::Escape => {
						if is_pressed {
							manager::close_pressed();
							return true;
						}
						false
					},
					Key::F1 => {
						if is_pressed {
							self.ui_manager.toggle_visibility();
							return true
						}
						false
					},
					Key::F11 => {
						if is_pressed {
							let window = self.window();
							
							if window.fullscreen().is_some() {
								// If already fullscreen, exit fullscreen
								window.set_fullscreen(None);
							} else {
								// Otherwise enter fullscreen
								let current_monitor = window.current_monitor().unwrap_or_else(|| {
									window.available_monitors().next().expect("No monitors available")
								});
								
								window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(current_monitor))));
							}
							return true;
						}
						false
					},
					_ => false,
				}
			},
			_ => false
		}
	}
	#[inline]
	pub fn handle_mouse_input(&mut self, event: &WindowEvent) -> bool {
		match event {
			WindowEvent::MouseInput { button, state, .. } => {
				match (button, *state) {
					(MouseButton::Left, ElementState::Pressed) => {
						self.input_system.mouse_button_state.left = true;
						if self.ui_manager.visibility == true {
							// Use the stored current mouse position
							if let Some(current_position) = self.input_system.previous_mouse {
								let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
								self.ui_manager.handle_ui_click(x, y, true);
							}
						}
						true
					}
					(MouseButton::Left, ElementState::Released) => {
						self.input_system.mouse_button_state.left = false;
						if self.ui_manager.visibility == true {
							// Use the stored current mouse position
							if let Some(current_position) = self.input_system.previous_mouse {
								let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
								self.ui_manager.handle_ui_click(x, y, false);
							}
						}
						true
					}
					(MouseButton::Right, ElementState::Pressed) => {
						self.input_system.mouse_button_state.right = true;
						true
					}
					(MouseButton::Right, ElementState::Released) => {
						self.input_system.mouse_button_state.right = false;
						true
					}
					_ => false,
				}
			}
			WindowEvent::CursorMoved { position, .. } => {
				if self.input_system.mouse_captured() {
					// Calculate relative movement from center
					let size = self.size();
					let center_x = size.width as f64 / 2.0;
					let center_y = size.height as f64 / 2.0;
					
					let delta_x = (position.x - center_x) as f32;
					let delta_y = (position.y - center_y) as f32;
					
					// Process mouse movement for camera control
					let gamestate = ptr::get_gamestate();
					if self.is_world_running && gamestate.is_running() {
						gamestate.player_mut().controller().process_mouse(delta_x, delta_y);
					}
					// Reset cursor to center
					self.center_mouse();
					self.input_system.previous_mouse = Some(winit::dpi::PhysicalPosition::new(center_x, center_y));
					return true;
				} else {
					let (x, y) = convert_mouse_position(self.render_context.size.into(), position);
					// Handle normal mouse movement for UI
					if self.ui_manager.visibility == true {
						self.ui_manager.handle_mouse_move(x, y, self.input_system.mouse_button_state.left);
					}
					
					// Handle UI hover
					self.ui_manager.handle_ui_hover(x, y);
					self.input_system.previous_mouse = Some(*position);
					return true;
				}

			}
			WindowEvent::MouseWheel { delta, .. } => {
				if self.is_world_running && ptr::get_gamestate().is_running() {
					let delta = match delta {
						MouseScrollDelta::LineDelta(_, y) => y * -0.5,
						MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * -0.01,
					}; // delta is reversed for some reason ... might need to look into it more (maybe it's different for platforms so yeah)
					let state = ptr::get_state();
					match state.ui_manager.state.clone() {
						manager::UIState::Inventory(_) => {
							// item interaction i guess
						},
						manager::UIState::InGame => {
							ptr::get_gamestate().player_mut().inventory_mut().step_select_slot(delta);
							ptr::get_state().ui_manager.setup_ui();
						}
						_ => return false,
					}
				}
				true
			}
			_ => false,
		}
	}
	pub fn converted_mouse_position(&self) -> (f32, f32) {
		convert_mouse_position(self.render_context.size.into(), &self.input_system.previous_mouse.unwrap_or(winit::dpi::PhysicalPosition::new(0.0, 0.0)))
	}

}


#[inline]
pub const fn convert_mouse_position(window_size: (u32, u32), mouse_pos: &winit::dpi::PhysicalPosition<f64>) -> (f32, f32) {
	let (x, y) = (mouse_pos.x as f32, mouse_pos.y as f32);
	let (width, height) = (window_size.0 as f32, window_size.1 as f32);
	((2.0 * x / width) - 1.0, (2.0 * (height - y) / height) - 1.0)
}