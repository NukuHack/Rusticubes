
use crate::{
	ext::{audio, ptr},
	ui::{
		manager::{UIManager, UIState, FocusState},
		element::UIElementData,
	},
	item::ui_inventory::ClickResult
};

impl UIManager {
	#[inline]
	fn handle_click_press(&mut self, x: f32, y: f32) -> bool {
		match self.state.clone() {
			UIState::Inventory(inv_state) => {
				let inv = ptr::get_gamestate().player_mut().inventory_mut();
				
				let Some(inv_lay) = inv.layout.clone() else { return false; };
				let click_result = inv_lay.handle_click(inv_state, x, y);
					
				let ClickResult::SlotClicked { area_type, slot } = click_result else { return false; };

				inv.handle_click_press(slot, area_type);
				self.setup_ui();
				return true;
			}
			_ => {
				self.clear_focused_element();
			}
		}
		// First collect the IDs of elements that need to be processed
		let mut candidate_ids: Vec<(usize, i32)> = self.elements.iter()
			.enumerate()
			.filter(|(_, e)| e.visible && e.enabled)
			.map(|(i, e)| (i, e.z_index))
			.collect();

		// Sort by z_index (highest first since we'll process in reverse)
		candidate_ids.sort_by_key(|(_, z)| -z);

		for (i, _) in candidate_ids {
			let element = &mut self.elements[i];
			if !element.contains_point(x, y) { continue; }

			let id = element.id; // Copy the ID first
			let focus_state:FocusState = match element.data {
				UIElementData::Checkbox{..} | 
				UIElementData::Button{..} |
				UIElementData::MultiStateButton{..} |
				UIElementData::Slider{..} => {
					element.set_calc_value(x, y);
					audio::set_fg("click.ogg");
					FocusState::Simple { id }
				},
				UIElementData::InputField{..}  => {
					audio::set_fg("click.ogg");
					FocusState::input(id)
				},
				_=> FocusState::default(),
			};

			if focus_state.is_none() { continue; }

			self.set_focused_state(focus_state);
			return true;
		}
		false
	}
	#[inline]
	fn handle_rclick_press(&mut self, x: f32, y: f32) -> bool {
		match self.state.clone() {
			UIState::Inventory(inv_state) => {
				let inv = ptr::get_gamestate().player_mut().inventory_mut();
				let Some(inv_lay) = inv.layout.clone() else { return false; };

				let click_result = inv_lay.handle_click(inv_state, x, y);
				
				let ClickResult::SlotClicked { area_type, slot } = click_result else { return false; };

				inv.handle_rclick_press(slot, area_type);
				self.setup_ui();
				return true;
			}
			_ => {
				self.clear_focused_element();
			}
		}
		return false;
	}
	#[inline]
	fn handle_mclick_press(&mut self, x: f32, y: f32) -> bool {
		match self.state.clone() {
			UIState::Inventory(inv_state) => {
				let inv = ptr::get_gamestate().player_mut().inventory_mut();
				let Some(inv_lay) = inv.layout.clone() else { return false; };

				let click_result = inv_lay.handle_click(inv_state, x, y);
				
				let ClickResult::SlotClicked { area_type, slot } = click_result else { return false; };

				inv.handle_mclick_press(slot, area_type);
				self.setup_ui();
				return true;
			}
			_ => {
				self.clear_focused_element();
			}
		}
		return false;
	}
	#[inline]
	fn handle_click_release(&mut self, x: f32, y: f32) -> bool {
		let Some(element) = self.get_focused_element_mut() else { return false; };

		if !element.contains_point(x, y) { return false; };

		match &element.data {
			UIElementData::InputField{..} |
			UIElementData::Checkbox{..} |
			UIElementData::Button{..} |
			UIElementData::MultiStateButton{..} |
			UIElementData::Slider{..} => {

				element.toggle_checked(); // only checkbox

				element.next_state(); // only multi button

				element.set_calc_value(x, y); // only slider
			},
			_ => {
				return false;
			},
		}
		element.trigger_callback();

		return true;
		// clearing the focused element is bad for text input, because you can't input then
	}

	pub fn handle_mouse_move(&mut self, x: f32, y: f32, is_pressed: bool) {
		// First check the conditions that don't need the element
		if matches!(self.state, UIState::Inventory(_)) {
			let inventory = ptr::get_gamestate().player_mut().inventory_mut();
			let Some(item) = inventory.get_cursor() else { return; };

			self.cursor_item_display(x,y,item);
		}
		// Then get the element
		let Some(element) = self.get_focused_element_mut() else { return };

		if let UIElementData::Slider{..} = element.data {
			if !is_pressed { return; } // slider only processes if the button is pressed
			element.set_calc_value(x, y);
			element.trigger_callback();
		}
	}

	pub fn handle_scroll(&mut self, delta: f32) -> bool {
		match self.state.clone() {
			UIState::Inventory(_) => {
				// item interaction i guess
				true
			},
			UIState::InGame => {
				let inventory = ptr::get_gamestate().player_mut().inventory_mut();
				inventory.step_select_slot(delta);
				
				self.hotbar_selection_highlight(inventory);

				true
			}
			_ => false,
		}
	}
	
	#[inline]
	pub fn handle_hover(&mut self, x: f32, y:f32) {
		// Update all elements
		self.elements
			.iter_mut()
			.for_each(|e| e.update_hover_state(e.contains_point(x, y)));
	}
	
	#[inline]
	pub fn handle_click(&mut self, x: f32, y:f32, pressed: bool) -> bool {
		if pressed {
			self.handle_click_press(x, y)
		} else {
			self.handle_click_release(x, y)
		}
	}
	#[inline]
	pub fn handle_rclick(&mut self, x: f32, y:f32, pressed: bool) -> bool {
		if pressed {
			self.handle_rclick_press(x, y)
		} else {
			//self.handle_rclick_release(x, y)
			false
		}
	}
	#[inline]
	pub fn handle_mclick(&mut self, x: f32, y:f32, pressed: bool) -> bool {
		if pressed {
			self.handle_mclick_press(x, y)
		} else {
			//self.handle_mclick_release(x, y)
			false
		}
	}
}
