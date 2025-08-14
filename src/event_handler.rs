
use crate::ext::{ptr, memory};
use crate::block::extra;
use crate::ui::manager::{self, UIState};
use crate::item::ui_inventory::{InventoryUIState};
use std::iter::Iterator;
use winit::{
	event::{ElementState, MouseButton, WindowEvent, MouseScrollDelta},
	keyboard::KeyCode as Key,
};

impl<'a> crate::State<'a> {

	#[inline]
	pub fn handle_events(&mut self, event: &WindowEvent) -> bool{

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
			WindowEvent::Occluded(is_hidden) => {
				if !is_hidden {return true;}
				// here it should clean up stuff, and also make the rendering basically non existant
				memory::light_trim();
				memory::hard_clean(Some(ptr::get_state().device()));
				true
			},
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
				let change = self.input_system.compare(&modifiers.state());
				self.input_system.modifiers = modifiers.state();
				if change.alt_key() {
					if self.input_system.modifiers.alt_key() {
						self.toggle_mouse_capture();
					}
					self.center_mouse();
				}
				true
			},
			WindowEvent::KeyboardInput { is_synthetic, .. } => {
				if *is_synthetic { return true; }
				self.handle_key_input(event);
				true
			},
			WindowEvent::MouseInput { .. } => {
				self.handle_mouse_input(event);
				true
			},
			WindowEvent::CursorMoved { .. } => {
				self.handle_mouse_move(event);
				true
			},
			WindowEvent::MouseWheel { .. } => {
				self.handle_mouse_scroll(event);
				true
			},
			_ => false,
		}
	}
	#[inline]
	pub fn handle_key_input(&mut self, event: &WindowEvent) -> bool {
		let WindowEvent::KeyboardInput {
			event: winit::event::KeyEvent {
				physical_key,
				state, // ElementState::Released or ElementState::Pressed
				/*rest of the booring stuff*/ logical_key, text: _, location: _, repeat: _, ..}, device_id: _, is_synthetic 
			} = event else { return false; };

		let input_str = logical_key.to_text().unwrap_or("");

		if *is_synthetic { return true; }
			
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
		if self.ui_manager.visibility {
			if let Some(element) = self.ui_manager.get_focused_element() { // if focused you can't press Esc, have to handle them in a custom way
				if self.is_world_running && element.is_input() {
					ptr::get_gamestate().player_mut().controller().reset_keyboard(); // Temporary workaround
				}
				if is_pressed {
					// Handle keys for UI
					if self.ui_manager.handle_keyboard_input(key, input_str) {
						return true;
					}
				}
			}
		}
		// Handle game controls if no UI element is focused
		// `key` is of type `KeyCode` (e.g., KeyCode::W)
		// `state` is of type `ElementState` (Pressed or Released)
		if self.is_world_running && ptr::get_gamestate().is_running() {
			ptr::get_gamestate().player_mut().controller().process_keyboard(&key, is_pressed);
			match key {
				Key::KeyG => {
					if !is_pressed { return false; }

					extra::add_full_chunk();
					return true
				},
				Key::KeyE => {
					if !is_pressed { return false; }

					match self.ui_manager.state.clone() {
						UIState::Inventory(_) => {
							manager::close_pressed();
							if !self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
						},
						UIState::InGame => {
							self.ui_manager.state = UIState::Inventory(InventoryUIState::default());
							if self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
						}
						_ => return false,
					}
					self.ui_manager.setup_ui();
					return true
				},
				Key::KeyI => {
					if !is_pressed { return false; }

					if self.ui_manager.state.clone() == UIState::InGame {
						self.ui_manager.state = UIState::Inventory(InventoryUIState::str().b());
						if self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
					} else { return false; }

					self.ui_manager.setup_ui();
					return true
				},
				Key::KeyO => {
					if !is_pressed { return false; }

					if self.ui_manager.state.clone() == UIState::InGame {
						self.ui_manager.state = UIState::Inventory(InventoryUIState::craft().b());
						if self.input_system.mouse_captured() { self.toggle_mouse_capture(); }
					} else { return false; }

					self.ui_manager.setup_ui();
					return true
				},
				_ => { },
			};
		}
		match key {
			// handling alt and shift block interaction is considered a "modifier button change" so it should be in the input system
			Key::Escape => {
				if !is_pressed { return false; }

				manager::close_pressed();
				return true;
			},
			Key::F1 => {
				if !is_pressed { return false; }

				self.ui_manager.toggle_visibility();
				return true
			},
			Key::F11 => {
				if !is_pressed { return false; }

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
			},
			_ => false,
		}
	}
	#[inline]
	pub fn handle_mouse_input(&mut self, event: &WindowEvent) -> bool {
		let WindowEvent::MouseInput { button, state, device_id: _ } = event else { return false; };

		match (button, *state) {
			(MouseButton::Left, ElementState::Pressed) => {
				self.input_system.mouse_button_state.left = true;
				if self.input_system.mouse_captured() {
					extra::remove_targeted_block();
					return true
				}
				if self.ui_manager.visibility {
					// Use the stored current mouse position
					if let Some(current_position) = self.input_system.previous_mouse {
						let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
						self.ui_manager.handle_click(x, y, true);
					}
				}
				true
			}
			(MouseButton::Left, ElementState::Released) => {
				self.input_system.mouse_button_state.left = false;
				if self.ui_manager.visibility {
					// Use the stored current mouse position
					if let Some(current_position) = self.input_system.previous_mouse {
						let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
						self.ui_manager.handle_click(x, y, false);
					}
				}
				true
			}
			(MouseButton::Right, ElementState::Pressed) => {
				self.input_system.mouse_button_state.right = true;
				if self.input_system.mouse_captured() {
					handle_right_click_interaction();
					return true
				}
				if self.ui_manager.visibility {
					// Use the stored current mouse position
					if let Some(current_position) = self.input_system.previous_mouse {
						let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
						self.ui_manager.handle_rclick(x, y, true);
					}
				}
				true
			}
			(MouseButton::Right, ElementState::Released) => {
				self.input_system.mouse_button_state.right = false;
				if self.ui_manager.visibility {
					// Use the stored current mouse position
					if let Some(current_position) = self.input_system.previous_mouse {
						let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
						self.ui_manager.handle_rclick(x, y, false);
					}
				}
				true
			}
			(MouseButton::Middle, ElementState::Pressed) => {
				self.input_system.mouse_button_state.middle = true;
				if self.ui_manager.visibility {
					// Use the stored current mouse position
					if let Some(current_position) = self.input_system.previous_mouse {
						let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
						self.ui_manager.handle_mclick(x, y, true);
					}
				}
				true
			},
			(MouseButton::Middle, ElementState::Released) => {
				self.input_system.mouse_button_state.middle = false;
				if self.ui_manager.visibility {
					// Use the stored current mouse position
					if let Some(current_position) = self.input_system.previous_mouse {
						let (x, y) = convert_mouse_position(self.render_context.size.into(), &current_position);
						self.ui_manager.handle_mclick(x, y, false);
					}
				}
				true
			},
			(MouseButton::Back, ElementState::Pressed) => {
				self.input_system.mouse_button_state.back = false;
				true
			},
			(MouseButton::Back, ElementState::Released) => {
				self.input_system.mouse_button_state.back = false;
				true
			},
			(MouseButton::Forward, ElementState::Pressed)  => {
				self.input_system.mouse_button_state.forward = false;
				true
			},
			(MouseButton::Forward, ElementState::Released)  => {
				self.input_system.mouse_button_state.forward = false;
				true
			},
			(MouseButton::Other(_), _) => false, // for now nothing needs this 
		}
	}
	#[inline]
	pub fn handle_mouse_move(&mut self, event: &WindowEvent) -> bool {
		let WindowEvent::CursorMoved { position, device_id: _ } = event else { return false; };

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
			if self.ui_manager.visibility {
				self.ui_manager.handle_mouse_move(x, y, self.input_system.mouse_button_state.left);
			}
			self.input_system.previous_mouse = Some(*position);
			return true;
		}
	}
	#[inline]
	pub fn handle_mouse_scroll(&mut self, event: &WindowEvent) -> bool {
		let WindowEvent::MouseWheel { delta, phase: _, device_id: _ } = event else { return false; };

		if self.is_world_running && ptr::get_gamestate().is_running() {
			let delta = match delta {
				MouseScrollDelta::LineDelta(_, y) => y * -0.5,
				MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * -0.01,
			}; // delta is reversed for some reason ... might need to look into it more (maybe it's different for platforms so yeah)
			self.ui_manager.handle_scroll(delta);
		}
		true
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


pub fn handle_right_click_interaction() {
    use crate::ext::ptr;

    let state = ptr::get_state();
    if !state.is_world_running {
        return;
    }

    let player = &ptr::get_gamestate().player();

    if handle_block_and_item_interaction(player) { 
        return; 
    }

    let Some(item) = player.inventory().selected_item() else { return; };

    if item.is_block() {
        handle_block_placing(player, item);
        return;
    }
    if item.is_consumable() {
        remove_selected_item_from_inv(); // "consume" the item
        return;
    }
}

/// Places a cube on the face of the block the player is looking at
use crate::player::Player;
use crate::item::items::ItemStack;
fn handle_block_placing(player: &Player, item: &ItemStack) {
    use crate::block::extra::*;
    use crate::ext::ptr;
    use crate::block::math::ChunkCoord;
    use crate::block::main::{Block, Material};
    
    let world = &mut ptr::get_gamestate().world_mut();

    let Some((block_pos, normal)) = raycast_to_block(player.camera(), player, world, REACH) else { return; };

    let placement_pos = block_pos + normal;

    // Simple for loop to find block ID
    let block_id = get_block_id_from_item_name(item.name());

    remove_selected_item_from_inv();

    world.set_block(placement_pos, Block::new(Material(block_id)));
    update_chunk_mesh(world, ChunkCoord::from_world_pos(placement_pos));
}

fn handle_block_and_item_interaction(player: &Player) -> bool {
    use crate::block::extra::*;
    use crate::ext::ptr;
    // here handle the block interaction

    let world = &mut ptr::get_gamestate().world_mut();
    let Some((block_pos, _normal)) = raycast_to_block(player.camera(), player, world, REACH) else { return false; };
    let Some(storage) = world.get_storage(block_pos) else { return false; };
    println!("Storage found {:?}", storage);
    // here we would open the UI for the inventory
    true
}

#[inline] 
fn remove_selected_item_from_inv() {
    use crate::item::inventory::AreaType;

    let inv_mut = ptr::get_gamestate().player_mut().inventory_mut();
    let idx = inv_mut.selected_index();

    let hotbar = inv_mut.get_area_mut(AreaType::Hotbar);
    let Some(item) = hotbar.remove(idx) else { return; };
    hotbar.set(idx, item.remove_from_stack(1));

    ptr::get_state().ui_manager.setup_ui(); // to update the hotbar if changed
}
