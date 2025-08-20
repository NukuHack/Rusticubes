
use crate::{
	ext::{audio, ptr},
	ui::{
		manager::{UIManager, UIState, FocusState},
		element::{UIElement, UIElementData},
	},
	item::ui_inventory::ClickResult,
	utils::input::{ClickMode, Keyboard},
};
use winit::keyboard::{ModifiersState, KeyCode as Key};

impl UIManager {
	pub fn handle_keyboard_input(&mut self, key: Key, input_str: &str) -> bool {
		let focus_state = self.get_focused_state();
		if matches!(focus_state, FocusState::Input { .. }) {
			return self.handle_key_input_on_input_field(key, input_str);
		}
		// we do not handle other kinds because i don't think we need to ...

		false
	}

	fn handle_click_press(&mut self, x: f32, y: f32) {
		// Get visible, enabled elements sorted by descending z-index
		let active_elements: Vec<(usize, i32)> = {
			let mut temp = self.elements
				.iter()
				.enumerate()
				.filter(|(_, e)| e.visible && e.enabled)
				.map(|(i, e)| (i, e.z_index))
				.collect::<Vec<_>>();
			temp.sort_by_key(|&(_, z)| std::cmp::Reverse(z));
			temp
		};

		for (element_index, _z_index) in active_elements {
			let element: &mut UIElement = &mut self.elements[element_index];
			if !element.contains_point(x, y) { continue }

			let focus_state = match element.data {
				UIElementData::Checkbox { .. } |
				UIElementData::Button { .. } |
				UIElementData::MultiStateButton { .. } => {
					FocusState::Simple { id: element.id }
				}
				UIElementData::Slider { .. } => {
					element.set_calc_value(x, y);
					FocusState::Simple { id: element.id }
				}
				UIElementData::InputField { .. } => {
					element.handle_input_clicked(x, y)
				}
				_ => FocusState::default(),
			};

			if focus_state.is_some() {
				audio::set_fg("click.ogg");
				self.set_focused_state(focus_state);
				return
			}
		}
	}
	#[inline]
	fn handle_click_release(&mut self, x: f32, y: f32) {
		let Some(element) = self.get_focused_element_mut() else { return };

		if !element.contains_point(x, y) { return };

		match &element.data {
			UIElementData::Checkbox { .. } => element.toggle_checked(),
			UIElementData::MultiStateButton { .. } => element.next_state(),
			UIElementData::Slider { .. } => element.set_calc_value(x, y),
			_ => (),
		}
		element.trigger_callback();
	}
	
	#[inline]
	pub fn handle_mouse_click(&mut self, x: f32, y:f32, pressed: bool, modifiers: &ModifiersState, keyboard: &Keyboard, mode: ClickMode) {
		if pressed {
			if let UIState::Inventory(inv_state) = self.state {
				let inv = ptr::get_gamestate().player_mut().inventory_mut();
				
				let Some(inv_lay) = inv.layout.as_ref() else { return };
				let ClickResult::SlotClicked { area_type, slot } = inv_lay.handle_click(inv_state, x, y) else { return };
				inv.handle_click_press(slot, modifiers, keyboard.w, area_type, mode); // the faslse represents the "W is pressed" variable
				
				self.setup_ui();
				return
			}
			self.clear_focused_state();

			match mode {
				ClickMode::Left => self.handle_click_press(x, y),
				ClickMode::Right => {}, // self.handle_rclick_press(x, y),
				ClickMode::Middle => {}, // self.handle_mclick_press(x, y),
			}
			return
		}

		match mode {
			ClickMode::Left => self.handle_click_release(x, y),
			ClickMode::Right => {}, // self.handle_rclick_release(x, y),
			ClickMode::Middle => {}, // self.handle_mclick_release(x, y),
		}
	}

	#[inline]
	pub fn handle_mouse_move(&mut self, x: f32, y: f32, is_pressed: bool) {
		// Update all elements - this is for hover handling
		self.elements
			.iter_mut()
			.for_each(|e| e.update_hover_state(e.contains_point(x, y)));

		// First check the conditions that don't need the element
		if matches!(self.state, UIState::Inventory(_)) {
			let inventory = ptr::get_gamestate().player().inventory();
			let Some(item) = inventory.get_cursor() else { return; };

			self.cursor_item_display(x,y,item);
		}
		// Then get the element
		let Some(element) = self.get_focused_element_mut() else { return };

		if matches!(element.data, UIElementData::Slider{..}) {
			if !is_pressed { return; } // slider only processes if the button is pressed
			element.set_calc_value(x, y);
			element.trigger_callback();
		}
	}

	#[inline]
	pub fn handle_scroll(&mut self, delta: f32) -> bool {
		if matches!(self.state, UIState::InGame) {
			let inventory = ptr::get_gamestate().player_mut().inventory_mut();
			inventory.step_select_slot(delta);
			self.hotbar_selection_highlight(inventory);
			return true;
		}
		false
	}
}
