
use crate::utils::input::{Keyboard, ClickMode};
use crate::ext::{ptr, memory};
use crate::block::extra;
use crate::ui::manager::{self, UIState};
use crate::item::ui_inventory::InventoryUIState;
use std::iter::Iterator;
use winit::{
	event::{ElementState, MouseButton, WindowEvent, MouseScrollDelta},
	keyboard::KeyCode as Key, dpi::PhysicalPosition
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
						self.input_system.reset_keyboard();
						ptr::get_gamestate().player_mut().controller_mut().process_keyboard(self.input_system.keyboard()); // Temporary workaround
					}
					self.ui_manager.clear_focused_state();
				} else {
					if self.is_world_running && ptr::get_gamestate().is_running() {
						// idk some stuff on focus getting ?
					}
				}
				true
			},
			WindowEvent::ModifiersChanged(modifiers) => {
				let change = self.input_system.compare(&modifiers.state());
				self.input_system.set_modifiers(modifiers.state());
				if change.alt_key() {
					if self.input_system.modifiers().alt_key() {
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
		let is_pressed = *state == ElementState::Pressed;

		self.input_system.handle_key_input(key, is_pressed);

		// Handle UI input first if there's a focused element
		if self.ui_manager.visibility {
			if let Some(element) = self.ui_manager.get_focused_element() { // if focused you can't press Esc, have to handle them in a custom way
				if self.is_world_running && element.is_input() {
					self.input_system.reset_keyboard();
					ptr::get_gamestate().player_mut().controller_mut().process_keyboard(self.input_system.keyboard()); // Temporary workaround
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
			if matches!(self.ui_manager.state, UIState::InGame)  {
				ptr::get_gamestate().player_mut().controller_mut().process_keyboard(self.input_system.keyboard());
			} // only handle player movement if not in inventory ...
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
							if !self.input_system.is_mouse_captured() { self.toggle_mouse_capture(); }
						},
						UIState::InGame => {
							let game_state = &mut ptr::get_gamestate();
							game_state.player_mut().controller_mut().process_keyboard(&Keyboard::default());
							self.ui_manager.state = UIState::Inventory(InventoryUIState::default());
							if self.input_system.is_mouse_captured() { self.toggle_mouse_capture(); }
						}
						_ => return false,
					}
					self.ui_manager.setup_ui();
					return true
				},
				k if k >= Key::Digit1 && k <= Key::Digit9 => { // nice maching for enums, using them as simple u8
					if !is_pressed { return false; }
					if self.is_world_running && ptr::get_gamestate().is_running() {
						let slot = (k as u8 - Key::Digit1 as u8) as isize;
						let inv_mut = ptr::get_gamestate().player_mut().inventory_mut();
						inv_mut.select_slot(slot);
						self.ui_manager.setup_ui();
					}
					return true
				},
				Key::KeyR => {
					if !is_pressed { return false; }

					let game_state = &mut ptr::get_gamestate(); let play_mut = game_state.player_mut();

					match self.ui_manager.state.clone() {
						UIState::Inventory(_) => {
							manager::close_pressed();
							if !self.input_system.is_mouse_captured() { self.toggle_mouse_capture(); }
						},
						UIState::InGame => {
							play_mut.controller_mut().process_keyboard(&Keyboard::default());
							let storage = play_mut.inventory_mut().get_crafting_mut();
							self.ui_manager.state = UIState::Inventory(InventoryUIState::craft().input(storage.slots()).b());
							if self.input_system.is_mouse_captured() { self.toggle_mouse_capture(); }
						}
						_ => return false,
					}
					let storage = play_mut.inventory_mut().get_crafting_mut();
					let storage_ptr: *mut ItemContainer = storage;
					play_mut.inventory_mut().storage_ptr = Some(storage_ptr);

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
				return true;
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

		// Use the stored current mouse position
		let (x, y) = convert_mouse_position(self.render_context.size.into(), &self.input_system.previous_mouse());
		let pressed = *state == ElementState::Pressed;
		self.input_system.handle_mouse_event(*button, pressed, *self.input_system.previous_mouse());
		let mods = self.input_system.modifiers(); let keyboard = self.input_system.keyboard();

		if self.ui_manager.visibility {
			self.ui_manager.handle_mouse_click(x, y, pressed, mods, keyboard, ClickMode::from(*button));
		}
		match button {
			MouseButton::Left => {
				if pressed && self.input_system.is_mouse_captured() {
					self.handle_lclick_interaction();
					return true;
				}
			},
			MouseButton::Right => {
				if pressed && self.input_system.is_mouse_captured() {
					self.handle_rclick_interaction();
					return true;
				}
			},
			MouseButton::Middle => {
				if pressed && self.input_system.is_mouse_captured() {
					//self.handle_mclick_interaction(); // will have to make a Middle click interaction too (for picking the block)
					return true;
				}
			},
			MouseButton::Back => {},
			MouseButton::Forward => {},
			MouseButton::Other(_) => {},
		}
		true
	}
	#[inline]
	pub fn handle_mouse_move(&mut self, event: &WindowEvent) -> bool {
		let WindowEvent::CursorMoved { position, device_id: _ } = event else { return false; };

		if self.input_system.is_mouse_captured() {
			// Calculate relative movement from center
			let size = self.size();
			let center_x = size.width as f64 / 2.0;
			let center_y = size.height as f64 / 2.0;
			
			let delta_x = (position.x - center_x) as f32;
			let delta_y = (position.y - center_y) as f32;
			
			// Process mouse movement for camera control
			let gamestate = ptr::get_gamestate();
			if self.is_world_running && gamestate.is_running() {
				gamestate.player_mut().controller_mut().process_mouse(delta_x, delta_y);
			}
			// Reset cursor to center
			self.center_mouse();
			self.input_system.handle_mouse_move(PhysicalPosition::new(center_x, center_y));
			return true;
		} else {
			let (x, y) = convert_mouse_position(self.render_context.size.into(), position);
			// Handle normal mouse movement for UI
			if self.ui_manager.visibility {
				self.ui_manager.handle_mouse_move(x, y, self.input_system.mouse_button_state().left);
			}
			self.input_system.handle_mouse_move(*position);
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
		convert_mouse_position(self.render_context.size.into(), &self.input_system.previous_mouse())
	}

}


#[inline]
pub const fn convert_mouse_position(window_size: (u32, u32), mouse_pos: &PhysicalPosition<f64>) -> (f32, f32) {
	let (x, y) = (mouse_pos.x as f32, mouse_pos.y as f32);
	let (width, height) = (window_size.0 as f32, window_size.1 as f32);
	((2.0 * x / width) - 1.0, (2.0 * (height - y) / height) - 1.0)
}


use crate::player::Player;
use crate::item::items::ItemStack;
use crate::item::inventory::ItemContainer;
use crate::block::extra::*;
use crate::block::math::ChunkCoord;
use crate::block::main::{Block, Material};
const CRAFTING_BLOCK:&str = "crafting";
impl<'a> crate::State<'a> {
	pub fn handle_rclick_interaction(&mut self) {
		if !self.is_world_running {
			return;
		}

		let player = &ptr::get_gamestate().player();

		if self.handle_block_and_item_interaction(player) { 
			self.ui_manager.setup_ui(); // to update the hotbar if changed
			return;
		}

		let Some(item) = player.inventory().selected_item() else { return; };

		if item.is_block() {
			if self.handle_block_placing(player, item) {
				self.ui_manager.setup_ui(); // to update the hotbar if changed
				return;
			}
		}
		if item.is_consumable() {
			if self.remove_selected_item_from_inv() { // "consume" the item
				self.ui_manager.setup_ui(); // to update the hotbar if changed
				return;
			}
		}
	}
	pub fn handle_lclick_interaction(&mut self) {
		if !self.is_world_running {
			return;
		}
		let player = &ptr::get_gamestate().player();

		if self.handle_block_breaking(player) {
			self.ui_manager.setup_ui();
			return;
		}
	}

	/// Places a cube on the face of the block the player is looking at
	fn handle_block_placing(&mut self, player: &Player, item: &ItemStack) -> bool {
		let world = &mut ptr::get_gamestate().world_mut();

		let Some((block_pos, normal)) = raycast_to_block(player.camera(), player, world, REACH) else { return false; };

		let placement_pos = block_pos + normal;

		// Simple for loop to find block ID
		let block_id = get_block_id_from_item_name(item.name());

		if !self.remove_selected_item_from_inv() { return false;};

		world.set_block(placement_pos, Block::new(Material(block_id)));
		update_chunk_mesh(world, ChunkCoord::from_world_pos(placement_pos));
		true
	}
	fn handle_block_breaking(&mut self, player: &Player) -> bool {
		let world = &mut ptr::get_gamestate().world_mut();

		let Some((block_pos, _normal)) = raycast_to_block(player.camera(), player, world, REACH) else { return false; };

		let block = world.get_block(block_pos);
		let inv_mut = ptr::get_gamestate().player_mut().inventory_mut();
		{
			let block_id = block.material.inner();
			let item_name = get_item_name_from_block_id(block_id);

			inv_mut.add_item_anywhere(ItemStack::new(item_name).with_stack_size(1));
		}
		if let Some(storage) = world.get_storage(block_pos) {
			for item in storage.iter() {
				let Some(itm) = item else { continue; };
				inv_mut.add_item_anywhere(itm.clone());
			}
		}

		world.set_block(block_pos, Block::default());
		update_chunk_mesh(world, ChunkCoord::from_world_pos(block_pos));

		true
	}

	fn handle_block_and_item_interaction(&mut self, player: &Player) -> bool {
		let game_state = &mut ptr::get_gamestate(); let world = game_state.world_mut();
		let Some((block_pos, _normal)) = raycast_to_block(player.camera(), player, world, REACH) else { return false; };
		let block_mat = world.get_block(block_pos).material.inner();

		let Some(storage) = world.get_storage_mut(block_pos) else { return false; };
		if self.ui_manager.state.clone() == UIState::InGame {
			if &get_item_name_from_block_id(block_mat) == CRAFTING_BLOCK {
				self.ui_manager.state = UIState::Inventory(InventoryUIState::craft().input(storage.slots()).b());
			} else {
				self.ui_manager.state = UIState::Inventory(InventoryUIState::str().size(storage.slots()).b());
			}
			if self.input_system.is_mouse_captured() { self.toggle_mouse_capture(); }
		}

		let storage_ptr: *mut ItemContainer = storage;
		let inv_mut = game_state.player_mut().inventory_mut();
		inv_mut.storage_ptr = Some(storage_ptr);

		true
	}

	#[inline] fn remove_selected_item_from_inv(&self) -> bool {
		use crate::item::inventory::AreaType;

		let inv_mut = ptr::get_gamestate().player_mut().inventory_mut();
		let idx = inv_mut.selected_index();

		let hotbar = inv_mut.get_area_mut(AreaType::Hotbar);
		let Some(item) = hotbar.remove(idx) else { return false; };
		hotbar.set(idx, item.remove_from_stack(1));
		true
	}
}
