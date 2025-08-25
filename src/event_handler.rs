
use crate::utils::input::{Keyboard, ClickMode};
use crate::ext::{ptr, memory};
use crate::block::extra;
use crate::ui::manager::{self, UIState};
use crate::item::ui_inventory::InventoryUIState;
use std::iter::Iterator;
use winit::{
	event::{ElementState, MouseButton, WindowEvent, MouseScrollDelta},
	keyboard::KeyCode, dpi::{PhysicalPosition, PhysicalSize}
};

impl<'a> crate::State<'a> {
	pub fn handle_events(&mut self, event: &WindowEvent) {
		match event {
			WindowEvent::CloseRequested => ptr::close_app(),
			WindowEvent::Resized(physical_size) => { self.resize(*physical_size); },
			WindowEvent::ScaleFactorChanged { .. } => {
				// Handle DPI change
			},
			WindowEvent::Occluded(is_hidden) => {
				if !is_hidden { return }
				// here it should clean up stuff, and also make the rendering basically non existant
				memory::light_trim();
				memory::hard_clean(Some(ptr::get_state().device()));
			},
			WindowEvent::RedrawRequested => {
				self.window().request_redraw();
				self.update();
				match self.render() {
					Ok(_) => {},
					Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
						self.resize(*self.size());
					},
					Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
						println!("Surface error");
						ptr::close_app();
					}
					Err(wgpu::SurfaceError::Timeout) => {
						println!("Surface timeout");
					},
				}
			},
			WindowEvent::Focused(focused) => {
				if !focused {
					self.input_system.clear();
					if self.is_world_running {
						self.input_system.reset_keyboard();
						ptr::get_gamestate().player_mut().controller_mut().process_keyboard(self.input_system.keyboard()); // Temporary workaround
					}
					self.ui_manager.clear_focused_state();
				}
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
			},
			WindowEvent::KeyboardInput { is_synthetic, event: winit::event::KeyEvent {
					physical_key, state, // ElementState::Released or ElementState::Pressed
					logical_key, text: _, location: _, repeat: _, .. }, device_id: _ } => {
				if *is_synthetic { return }
				
				let winit::keyboard::PhysicalKey::Code(key) = physical_key else { println!("-WindowEvent-KeyboardInput error : no keyboard input"); return };
				let is_pressed = *state == ElementState::Pressed;
				let input_str = logical_key.to_text().unwrap_or("");
				self.handle_key_input(*key, is_pressed, input_str);
			},
			WindowEvent::MouseInput { button, state, device_id: _ } => self.handle_mouse_input(button, state),
			WindowEvent::CursorMoved { position, device_id: _ } => self.handle_mouse_movement(position),
			WindowEvent::MouseWheel { delta, phase: _, device_id: _ } => self.handle_mouse_scroll(delta),
			_ => {},
		}
	}
	#[inline] fn can_handle_game_input(&self) -> bool {
		self.is_world_running && ptr::get_gamestate().is_running()
	}
	#[inline] pub fn handle_key_input(&mut self, key: KeyCode, is_pressed: bool, input_str: &str) {
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
						return
					}
				}
			}
		}
		// Handle game controls if no UI element is focused
		// `key` is of type `KeyCode` (e.g., KeyCode::W)
		// `state` is of type `ElementState` (Pressed or Released)
		if self.can_handle_game_input() {
			if matches!(self.ui_manager.state, UIState::InGame)  {
				ptr::get_gamestate().player_mut().controller_mut().process_keyboard(self.input_system.keyboard());
			} // only handle player movement if not in inventory ...
			match key {
				KeyCode::KeyG => {
					if !is_pressed { return }

					extra::add_full_chunk();
					return
				},
				k if k >= KeyCode::Digit1 && k <= KeyCode::Digit9 => {
					if !is_pressed || !self.can_handle_game_input() { return }
					
					let slot = (k as u8 - KeyCode::Digit1 as u8) as isize;
					ptr::get_gamestate().player_mut().inventory_mut().select_slot(slot);
					self.ui_manager.setup_ui();
				},
				KeyCode::KeyE => {
					if !is_pressed { return }

					match self.ui_manager.state.clone() {
						UIState::Inventory(_) => self.close_inventory(),
						UIState::InGame => {
							self.transition_inventory_state(InventoryUIState::default());
						}
						_ => return,
					}
					self.ui_manager.setup_ui();
					return
				},
				KeyCode::KeyR => {
					if !is_pressed { return }

					let game_state = &mut ptr::get_gamestate(); let play_mut = game_state.player_mut();

					match self.ui_manager.state.clone() {
						UIState::Inventory(_) => self.close_inventory(),
						UIState::InGame => {
							let slots = play_mut.inventory().get_crafting().slots();
							self.transition_inventory_state(InventoryUIState::craft().input(slots).b());
						}
						_ => return,
					}
					let storage = play_mut.inventory_mut().get_crafting_mut();
					let storage_ptr: *mut ItemContainer = storage;
					play_mut.inventory_mut().storage_ptr = Some(storage_ptr);

					self.ui_manager.setup_ui();
					return
				},
				_ => { },
			};
		}
		match key {
			// handling alt and shift block interaction is considered a "modifier button change" so it should be in the input system
			KeyCode::Escape => {
				if !is_pressed { return }

				manager::close_pressed();
				return
			},
			KeyCode::F1 => {
				if !is_pressed { return }

				match self.ui_manager.state.clone() {
					UIState::Inventory(_) | 
					UIState::InGame => self.ui_manager.toggle_visibility(),
					_ => {},
				}
				return
			},
			KeyCode::Enter => {
				if !is_pressed || matches!(self.ui_manager.state, UIState::InGame) { return }

				if self.ui_manager.visibility {
					self.ui_manager.trigger_click_on_focused_element();
					self.ui_manager.setup_ui();
				}
				return
			},
			KeyCode::Tab => {
				if !is_pressed || matches!(self.ui_manager.state, UIState::InGame) { return }

				if self.ui_manager.visibility {
					self.ui_manager.select_next_element();
				}
				return
			},
			KeyCode::F11 => {
				if !is_pressed { return }

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
				return
			},
			KeyCode::F4 => { // auto implemented 'Alt + F4' closing ...
				if !is_pressed { return }

				if self.input_system.modifiers().alt_key() {
					ptr::close_app();
				}
				return
			},
			_ => {},
		}
	}
	/// Handles mouse movement and returns whether it was successfully processed
	#[inline] pub fn handle_mouse_movement(&mut self, position: &PhysicalPosition<f64>) {
		if !self.input_system.is_mouse_captured() {
			let (x, y) = convert_mouse_position(self.size(), position);
			
			// Handle normal mouse movement for UI
			if self.ui_manager.visibility {
				self.ui_manager.handle_mouse_move(x, y, self.input_system.mouse_button_state().left);
			}
			
			self.input_system.handle_mouse_move(*position);
			return
		}
		
		// Reset cursor to center for captured mouse
		self.center_mouse();

		// Calculate relative movement from center
		let pos = self.input_system.previous_mouse();
		
		let delta_x = (position.x - pos.x) as f32;
		let delta_y = (position.y - pos.y) as f32;
		
		// Process mouse movement for camera control if world is running
		if self.can_handle_game_input() {
			ptr::get_gamestate().player_mut().controller_mut().process_mouse(delta_x, delta_y);
		}
	}
	#[inline] pub fn handle_mouse_input(&mut self, button: &MouseButton, state: &ElementState) {
		// Use the stored current mouse position
		let (x, y) = convert_mouse_position(self.size(), &self.input_system.previous_mouse());
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
					return
				}
			},
			MouseButton::Right => {
				if pressed && self.input_system.is_mouse_captured() {
					self.handle_rclick_interaction();
					return
				}
			},
			MouseButton::Middle => {
				if pressed && self.input_system.is_mouse_captured() {
					//self.handle_mclick_interaction(); // will have to make a Middle click interaction too (for picking the block)
					return
				}
			},
			MouseButton::Back => {},
			MouseButton::Forward => {},
			MouseButton::Other(_) => {},
		}
	}
	#[inline] pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
		if self.can_handle_game_input() {
			let delta = match delta {
				MouseScrollDelta::LineDelta(_, y) => y * -0.5,
				MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * -0.01,
			}; // delta is reversed for some reason ... might need to look into it more (maybe it's different for platforms so yeah)
			self.ui_manager.handle_scroll(delta);
		}
	}

	#[inline] fn transition_inventory_state(&mut self, new_state: InventoryUIState) {
		let game_state = ptr::get_gamestate();
		game_state.player_mut().controller_mut().process_keyboard(&Keyboard::default());
		
		self.ui_manager.state = UIState::Inventory(new_state);
		
		if self.input_system.is_mouse_captured() { 
			self.toggle_mouse_capture(); 
		}
	}
	#[inline] fn close_inventory(&mut self) {
		manager::close_pressed();
		if !self.input_system.is_mouse_captured() { 
			self.toggle_mouse_capture(); 
		}
	}

	#[inline] pub fn converted_mouse_position(&self) -> (f32, f32) {
		convert_mouse_position(self.size(), &self.input_system.previous_mouse())
	}

}


#[inline] pub const fn convert_mouse_position(window_size: &PhysicalSize<u32>, mouse_pos: &PhysicalPosition<f64>) -> (f32, f32) {
	let (x, y) = (mouse_pos.x as f32, mouse_pos.y as f32);
	let (width, height) = (window_size.width as f32, window_size.height as f32);
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
		if !self.can_handle_game_input() { return }

		let player = &ptr::get_gamestate().player();

		if matches!(self.ui_manager.state, UIState::InGame) && self.handle_block_and_item_interaction(player) { 
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
		if !self.can_handle_game_input() { return }
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

			inv_mut.add_item_anywhere(&mut ItemStack::new(item_name).with_stack_size(1));
		}
		if block.is_storage() { if let Some(storage) = world.get_storage_mut(block_pos) {
			for item in storage.iter_mut() {
				let Some(itm) = item else { continue };
				inv_mut.add_item_anywhere(itm);
			}
		}}

		world.set_block(block_pos, Block::default());
		update_chunk_mesh(world, ChunkCoord::from_world_pos(block_pos));

		true
	}

	fn handle_block_and_item_interaction(&mut self, player: &Player) -> bool {
		let game_state = &mut ptr::get_gamestate(); let world = game_state.world_mut();
		let Some((block_pos, _normal)) = raycast_to_block(player.camera(), player, world, REACH) else { return false; };
		let block_mat = world.get_block(block_pos).material.inner();

		let Some(storage) = world.get_storage_mut(block_pos) else { return false; };
		self.transition_inventory_state(if &get_item_name_from_block_id(block_mat) == CRAFTING_BLOCK { 
			InventoryUIState::craft().input(storage.slots()).b() } else { InventoryUIState::str().size(storage.slots()).b() });

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
