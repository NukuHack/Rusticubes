
use winit::keyboard::ModifiersState;
use crate::item::ui_inventory::InventoryLayout;
use crate::item::items::ItemStack;
use crate::utils::input::ClickMode;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AreaType { 
	Panel,  Inventory,  Hotbar, 
	Armor,  Storage, Output 
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Slot { rows: u8, cols: u8 }

impl Slot {
	pub const NONE: Self = Self { rows: 0, cols: 0 };
	pub const TINY: Self = Self { rows: 3, cols: 5 };
	pub const SMALL: Self = Self { rows: 3, cols: 7 };
	pub const MED: Self = Self { rows: 5, cols: 9 };
	pub const BIG: Self = Self { rows: 6, cols: 12 };
	pub const GIANT: Self = Self { rows: 7, cols: 13 };

	#[inline] pub const fn default() -> Self { Self::SMALL }
	#[inline] pub const fn rows(&self) -> u8 { self.rows }
	#[inline] pub const fn cols(&self) -> u8 { self.cols }
	#[inline] pub const fn total(&self) -> usize { self.rows() as usize * self.cols() as usize }
	#[inline] pub const fn custom(rows: u8, cols: u8) -> Self { Self { rows, cols } }
}
impl From<(u8, u8)> for Slot {
	#[inline]
	fn from(tupl: (u8, u8)) -> Self {
		Self::custom(tupl.0, tupl.1)
	}
}

// Default inventory sizes - can be upgraded during gameplay
pub const DEFAULT_ARMOR_SLOTS: u8 = 4;
pub const DEFAULT_HOTBAR_SLOTS: u8 = 5;
pub const DEFAULT_INV_ROWS: u8 = 3;
pub const DEFAULT_INV_COLS: u8 = 7;

/// A unified container for items that can handle both 1D and 2D layouts
#[derive(Clone, PartialEq, Debug)]
pub struct ItemContainer {
	items: Vec<Option<ItemStack>>,
	size: Slot,
}
impl ItemContainer {
	#[inline] pub fn default() -> Self {
		Self::new(1,1)
	}
	/// Create a new 2D grid container (like main inventory)
	#[inline] pub fn new(rows: u8, cols: u8) -> Self {
		Self::with_dimensions(Slot::custom(rows, cols))
	}
	#[inline] pub fn with_dimensions(size: Slot) -> Self {
		let mut container = Self {
			items: Vec::new(),
			size,
		};
		container.resize_to_capacity();
		container
	}
	#[inline] pub fn from_raw(size: Slot, items: Vec<Option<ItemStack>>) -> Self {
		let to_resize = if size.total() > items.len() { true } else { false };
		let mut container = Self {
			items,
			size,
		};
		if to_resize { container.resize_to_capacity(); }
		container
	}

	/// Resize the container to match its capacity
	#[inline] 
	fn resize_to_capacity(&mut self) {
		self.items.resize(self.capacity(), None);
	}
	/// Get dimensions
	#[inline] pub const fn rows(&self) -> u8 { self.size.rows() }
	#[inline] pub const fn cols(&self) -> u8 { self.size.cols() }
	#[inline] pub const fn slots(&self) -> Slot { self.size }
	#[inline] pub const fn capacity(&self) -> usize { self.rows() as usize * self.cols() as usize }

	/// Check if this is a 1D container (single row or single column)
	#[inline] pub const fn is_linear(&self) -> bool {
		self.rows() == 1 || self.cols() == 1
	}

	/// Get the first
	#[inline] pub fn first(&self) -> Option<&ItemStack> {
		self.items.get(0)?.as_ref()
	}

	/// Get an item by linear index
	#[inline] pub fn get(&self, index: usize) -> Option<&ItemStack> {
		self.items.get(index)?.as_ref()
	}
	// SAFETY: Caller must ensure `index < self.capacity()`
	#[inline(always)]
	pub unsafe fn get_unchecked(&self, index: usize) -> Option<&ItemStack> { unsafe {
		self.items.get_unchecked(index).as_ref()
	}}
	/// Get item at grid position (row, col) - works for both 1D and 2D
	#[inline] 
	pub fn get_at(&self, row: u8, col: u8) -> Option<&ItemStack> {
		let index = self.calculate_index(row, col)?;
		self.get(index)
	}

	/// Set an item by linear index
	#[inline] pub fn set(&mut self, index: usize, item: Option<ItemStack>) -> bool {
		if let Some(slot) = self.items.get_mut(index) {
			*slot = item;
			true
		} else {
			false
		}
	}
	/// Set item at grid position (row, col) - works for both 1D and 2D
	#[inline] pub fn set_at(&mut self, row: u8, col: u8, item: Option<ItemStack>) -> bool {
		self.calculate_index(row, col)
			.map_or(false, |index| self.set(index, item))
	}

	/// Helper method to calculate linear index from 2D coordinates
	#[inline(always)]
	fn calculate_index(&self, row: u8, col: u8) -> Option<usize> {
		if self.is_linear() {
			// For linear containers, use whichever coordinate is non-zero
			let sum = row as usize + col as usize;
			if sum > self.capacity() { return None; }

			return Some(sum);
		}

		let mut r = row; let mut c = col;
		if r >= self.rows() || c >= self.cols() {
			r = col; c = row; // switch

			if r >= self.rows() || c >= self.cols() {
				return None;
			}
		}

		Some((r as usize * self.cols() as usize) + c as usize)
	}

	/// Find the first empty slot
	#[inline] pub fn find_empty_slot(&self) -> Option<usize> {
		self.items.iter().position(|slot| slot.is_none())
	}

	/// Count non-empty slots
	#[inline] pub fn count_items(&self) -> usize {
		self.items.iter().filter(|slot| slot.is_some()).count()
	}

	/// Check if the container is full
	#[inline] pub fn is_full(&self) -> bool {
		self.find_empty_slot().is_none()
	}

	// In your sub-container implementation:
	#[inline]
	pub fn add_item(&mut self, item: &mut ItemStack) -> bool {
		if item.stack == 0 { return true; }
		// First try to stack with existing items
		for slot in self.iter_mut() {
			let Some(existing_item) = slot else { continue; };

			if !existing_item.can_stack_with(item) { continue; }

			let remaining = existing_item.add_to_stack(item.stack);
			item.stack = remaining;
			
			if item.stack == 0 { return true; } // Fully placed
		}

		// Try to find an empty slot
		if let Some(index) = self.find_empty_slot() {
			let mut new_item = item.clone();
			new_item.set_stack_size(item.stack.min(new_item.max_stack_size()));
			self.set(index, Some(new_item.clone()));
			item.stack -= new_item.stack;
			return true;
		}
		
		false
	}

	/// Remove an item at the specified linear index
	#[inline] pub fn remove(&mut self, index: usize) -> Option<ItemStack> {
		self.items.get_mut(index)?.take()
	}
	/// Remove an item at grid position (row, col)
	#[inline] pub fn remove_at(&mut self, row: u8, col: u8) -> Option<ItemStack> {
		let index = self.calculate_index(row, col)?;
		self.remove(index)
	}

	/// Upgrade the dimensions of this container
	#[inline] pub fn resize(&mut self, rows: u8, cols: u8) {
		self.size = Slot::custom(rows, cols);
		self.resize_to_capacity();
	}

	/// Upgrade capacity by finding appropriate dimensions (works for both linear and 2D containers)
	#[inline] pub fn expand(&mut self, new_capacity: u8) {
		if new_capacity == 0 {
			self.resize(0, 0);
			return;
		}

		if self.is_linear() && self.capacity() < 15 {
			if self.rows() == 1 {
				self.resize(1, new_capacity);
			} else {
				self.resize(new_capacity, 1);
			}
			return;
		}

		let (new_rows, new_cols) = Self::optimal_dimensions(new_capacity);
		self.resize(new_rows, new_cols);
	}

	/// Calculates the most balanced dimensions for given capacity
	fn optimal_dimensions(capacity: u8) -> (u8, u8) {
		match capacity {
			0..=9 => (1, capacity), // Small capacities work best as single row
			_ => {
				let mut best = (1, capacity);
				let mut min_diff = capacity as i32 - 1;

				for i in 1..=(capacity as f32).sqrt().ceil() as u8 {
					if capacity % i != 0 { continue; };

					let j = capacity / i;
					let diff = (i as i32 - j as i32).abs();
					if diff < min_diff {
						min_diff = diff;
						best = (i, j);
					}
				}

				// Prefer wider containers (more columns than rows)
				if best.0 > best.1 {
					(best.1, best.0)
				} else {
					best
				}
			}
		}
	}

	#[inline] pub fn items(&self) -> &[Option<ItemStack>] { &self.items }
	#[inline] pub fn size(&self) -> &Slot { &self.size }
	#[inline] pub fn clear(&mut self) {
		for slot in &mut self.items {
			*slot = None;
		}
	}
	
	/// Returns the smallest stack size in the inventory (ignores None slots)
	#[inline] pub fn smallest_stack_size(&self) -> u32 {
		self.iter()
			.filter_map(|slot| slot.as_ref().map(|stack| stack.stack()))
			.min()
			.unwrap_or(0)
	}

	#[inline] pub fn iter(&self) -> impl Iterator<Item = &Option<ItemStack>> {
		self.items.iter()
	}
	#[inline] pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Option<ItemStack>> {
		self.items.iter_mut()
	}
	// New into_iter() that consumes self
	#[inline] pub fn into_iter(self) -> impl Iterator<Item = Option<ItemStack>> {
		self.items.into_iter()  // Requires self (ownership)
	}
}

#[derive(Clone, PartialEq)]
pub struct Inventory {
	selected_slot: usize,
	// Armor slots (helmet, chestplate, leggings, boots, etc.) 
	armor: ItemContainer,
	// Main inventory grid - 2D grid
	items: ItemContainer,
	// Quick access hotbar 
	hotbar: ItemContainer,
	// clicked item
	cursor_item: Option<ItemStack>,
	// the payout
	pub layout: Option<InventoryLayout>,
	// Pointer for the in world Machine / Storage
	pub storage_ptr: Option<*mut ItemContainer>,
	// Basic inventory crafting grid to make basic stuff
	pub crafting_def: ItemContainer,

	/*
	// Consider:
	use std::rc::{Rc, Weak};
	use std::cell::RefCell;
	*/
}

impl Inventory {
	pub fn default() -> Self {
		Self::new(
			DEFAULT_ARMOR_SLOTS, 
			DEFAULT_INV_ROWS, 
			DEFAULT_INV_COLS, 
			DEFAULT_HOTBAR_SLOTS
		)
	}

	/// Create a new inventory with custom dimensions
	#[inline] pub fn new(armor_slots: u8, rows: u8, cols: u8, hotbar_slots: u8) -> Self {
		Self {
			selected_slot: 0,
			armor: ItemContainer::new(armor_slots, 1),
			items: ItemContainer::new(rows, cols),
			hotbar: ItemContainer::new(1, hotbar_slots),
			crafting_def: ItemContainer::new(2,2), // to make the player able to craft the crafting table ...
			cursor_item: None,
			layout: None,
			storage_ptr: None,
		}
	}

	/// Create a new inventory with custom dimensions
	#[inline] pub fn from_raw(armor: ItemContainer, items: ItemContainer, hotbar: ItemContainer, crafting_def: ItemContainer, storage_ptr: Option<*mut ItemContainer>) -> Self {
		Self {
			selected_slot: 0,
			armor,
			items,
			hotbar,
			crafting_def,
			cursor_item: None,
			layout: None,
			storage_ptr,
		}
	}
	
	// Getters for compatibility
	#[inline] pub const fn armor_size(&self) -> (u8,u8) { (self.armor.rows(),self.armor.cols()) }
	#[inline] pub const fn hotbar_size(&self) -> (u8,u8) { (self.hotbar.rows(),self.hotbar.cols()) }
	#[inline] pub const fn inv_size(&self) -> (u8,u8) { (self.items.rows(),self.items.cols()) }
	#[inline] pub const fn armor(&self) -> &ItemContainer { &self.armor }
	#[inline] pub const fn hotbar(&self) -> &ItemContainer { &self.hotbar }
	#[inline] pub const fn inv(&self) -> &ItemContainer { &self.items }
	#[inline] pub const fn selected_index(&self) -> usize { self.selected_slot }

	#[inline] pub fn selected_item(&self) -> Option<&ItemStack> {
		self.hotbar.get(self.selected_index())
	}
		
	/// Select a different hotbar slot
	#[inline] pub const fn select_slot(&mut self, idx: isize) {
		self.selected_slot = match idx {
			i if i >= self.hotbar.capacity() as isize => 0,
			i if i < 0 => self.hotbar.capacity() - 1,
			i => i as usize,
		};
	}
	#[inline] pub const fn step_select_slot(&mut self, delta: f32) {
		let step = if delta > 0.0 { 1 } else if delta < 0.0 { -1 } else { 0 };
		self.select_slot(self.selected_slot as isize + step);
	}

	#[inline] pub const fn get_cursor(&self) -> Option<&ItemStack> {
		self.cursor_item.as_ref()
	}
	#[inline] pub fn set_cursor(&mut self, item: Option<ItemStack>) {
		self.cursor_item = item;
	}
	#[inline] pub const fn remove_cursor(&mut self) -> Option<ItemStack> {
		self.cursor_item.take()
	}
	
	/// Set the UI layout
	#[inline] pub fn set_layout(&mut self, layout: &InventoryLayout) {
		self.layout = Some(layout.clone());
	}
	#[inline] pub const fn get_layout(&self) -> Option<&InventoryLayout> {
		self.layout.as_ref()
	}
	
	/// Get total item capacity
	#[inline] pub const fn total_capacity(&self) -> usize {
		self.armor.capacity() + self.hotbar.capacity() + self.items.capacity()
	}

	#[inline] pub fn get_area(&self, area: &AreaType) -> &ItemContainer {
		match area {
			AreaType::Inventory => &self.items,
			AreaType::Hotbar => &self.hotbar,
			AreaType::Armor => &self.armor,
			_ => {
				unsafe {
					if let Some(ptr) = self.storage_ptr {
						if !ptr.is_null() {
							return &mut *ptr;
						}
					}
					panic!("Invalid area type: {:?}", area)
				}
			}
		}
	}
	#[inline] pub fn get_area_mut(&mut self, area: AreaType) -> &mut ItemContainer {
		match area {
			AreaType::Inventory => &mut self.items,
			AreaType::Hotbar => &mut self.hotbar,
			AreaType::Armor => &mut self.armor,
			_ => {
				unsafe {
					if let Some(ptr) = self.storage_ptr {
						if !ptr.is_null() {
							return &mut *ptr;
						}
					}
					panic!("Invalid area type: {:?}", area)
				}
			}
		}
	}

	#[inline] pub fn is_self_pointing(&self) -> bool {
		// Get the pointer to our own crafting_def
		let self_ptr = &self.crafting_def as *const ItemContainer;
		
		// Check if storage_ptr matches our self pointer
		match self.storage_ptr {
			Some(ptr) if !ptr.is_null() => ptr as *const _ == self_ptr,
			_ => false
		}
	}
	#[inline] pub const fn get_crafting_mut(&mut self) -> &mut ItemContainer {
		&mut self.crafting_def
	}
	#[inline] pub const fn get_crafting(&self) -> &ItemContainer {
		&self.crafting_def
	}

	/// Add item to any available slot (tries hotbar first, then inventory, then armor - if is an armor item)
	#[inline]
	pub fn add_item_anywhere(&mut self, item: &mut ItemStack) -> bool {
		let initial_stack = item.stack;
		
		// Try to add to hotbar first
		if self.hotbar.add_item(item) {
			return true;
		}
		
		// Try inventory
		if self.items.add_item(item) {
			return true;
		}
		
		// Try armor if applicable
		if item.is_armor() && self.armor.add_item(item) {
			return true;
		}
		
		// Return true if any items were placed, false if none
		item.stack < initial_stack
	}

	/// Count total items across all containers
	#[inline] pub fn total_count(&self) -> usize {
		self.armor.count_items() + self.hotbar.count_items() + self.items.count_items()
	}
	#[inline] pub fn is_full(&self) -> bool {
		self.armor.is_full() && self.hotbar.is_full() && self.items.is_full()
	}

	pub fn make_result_from_input(&self) -> Option<ItemContainer> {
		let crafting_items = self.get_area(&AreaType::Storage);
		let Some(layout) = self.get_layout() else { return None; };
		let Some(result_area) = layout.areas.iter().find(|a| a.name == AreaType::Output) else { return None; };

		let Some(result) = crafting_items.find_recipe() else { return None; };
		let items = result.to_item_vec();

		if items.len() > result_area.capacity() || items.len() == 0 { 
			return None; 
		};

		let mut item_cont = ItemContainer::new(result_area.rows, result_area.cols);
		for (i, item_slot) in item_cont.iter_mut().enumerate() {
			let Some(item) = items.get(i) else { continue };
			*item_slot = item.clone().opt();
		}
		Some(item_cont)
	}
}

impl Inventory {
	/// Handles left click - full item interactions
	pub fn handle_click_press(&mut self, clicked_pos: (u8, u8), modifiers: &ModifiersState, do_extra: bool, area_type: AreaType, mode: ClickMode) {
		let shift = modifiers.shift_key();
		if matches!(area_type, AreaType::Output) {
			self.handle_output_click(clicked_pos, shift, mode);
			return;
		}
		
		let cursor = self.get_cursor().cloned();
		let (c_x, c_y) = clicked_pos;
		
		if shift {
			self.handle_shift_click(cursor, c_x, c_y, area_type, mode, do_extra);
			return;
		}
		
		self.handle_normal_click(cursor, c_x, c_y, area_type, mode);
	}

	// ===== CORE LOGIC METHODS =====

	/// Handles output area clicks (crafting results)
	fn handle_output_click(&mut self, clicked_pos: (u8, u8), shift: bool, click_type: ClickMode) {
		let cursor = self.get_cursor().cloned();
		let (c_x, c_y) = clicked_pos;
		
		let Some(result_container) = self.make_result_from_input() else { return };
		let Some(result_item) = result_container.get_at(c_x, c_y) else { return };
		
		if shift {
			let input_area = self.get_area(&AreaType::Storage);
			// Shift-click: try to add result items directly to inventory
			let count = match click_type {
				ClickMode::Left => input_area.smallest_stack_size().max(1),
				ClickMode::Right => (input_area.smallest_stack_size().max(1) + 1) / 2, // Round up division
				ClickMode::Middle => result_item.max_stack_size(),
			};
			
			if count == 0 { return; }
			let mut item = result_item.clone().with_stack_size(count);
			if self.add_item_anywhere(&mut item) && click_type != ClickMode::Middle {
				self.consume_crafting_materials(count);
			}
			return;
		}
		
		// Handle normal clicks (similar to handle_normal_click structure)
		match (cursor, click_type) {
			// Case 1: Empty cursor - pick up result item
			(None, ClickMode::Left) => {
				self.set_cursor(result_item.clone().opt());
				self.consume_crafting_materials(1);
			},
			(None, ClickMode::Right) => {
				self.set_cursor(result_item.clone().opt());
				self.consume_crafting_materials(1);
			},
			(None, ClickMode::Middle) => {
				let mut item = result_item.clone();
				item.set_to_max_stack();
				self.set_cursor(item.opt());
				// Don't consume materials for middle-click (creative mode)
			},
			
			// Case 2: Cursor has item - try to stack or reject
			(Some(mut cursor_item), ClickMode::Left) => {
				if cursor_item.can_stack_with(result_item) {
					let remaining = cursor_item.add_to_stack(result_item.stack());
					if remaining == 0 {
						// Successfully added to cursor stack
						self.set_cursor(cursor_item.opt());
						self.consume_crafting_materials(result_item.stack());
					}
					// If remaining > 0, cursor is full, do nothing
				}
				// If items can't stack, do nothing (can't place items in output)
			},
			(Some(cursor_item), ClickMode::Right) => {
				if cursor_item.can_stack_with(result_item) {
					let mut new_cursor = cursor_item.clone();
					let remaining = new_cursor.add_to_stack(1);
					if remaining == 0 {
						// Successfully added 1 to cursor stack
						self.set_cursor(new_cursor.opt());
						self.consume_crafting_materials(1);
					}
					// If remaining > 0, cursor is full, do nothing
				}
				// If items can't stack, do nothing
			},
			(Some(_), ClickMode::Middle) => {
				// Middle-click with cursor item in output area - do nothing
				// (consistent with not being able to place items in output)
			},
		}
	}

	/// Handles shift-click behavior (move items between areas)
	fn handle_shift_click(&mut self, cursor: Option<ItemStack>, c_x: u8, c_y: u8, area_type: AreaType, mode: ClickMode, do_extra: bool) {
		// extra will be used to click into the "non inventory container" like chest / crafting input
		
		let target_area = {
			if do_extra && area_type != AreaType::Storage && self.storage_ptr.is_some() { AreaType::Storage }
			else if area_type == AreaType::Inventory { AreaType::Hotbar }
			else { AreaType::Inventory }
		};
		let area = self.get_area_mut(area_type);
		
		match (cursor, area.remove_at(c_x, c_y)) {
			// Case 1: some in cursor but non in inventory : do the basic click (like it would without shift)
			(Some(cursor_item), None) => {
				let item_to_place = match mode {
					ClickMode::Right => {
						let single_item = cursor_item.clone().with_stack_size(1);
						let remaining = cursor_item.remove_from_stack(1);
						area.set_at(c_x, c_y, single_item.opt());
						self.set_cursor(remaining);
						return;
					},
					ClickMode::Left | ClickMode::Middle => cursor_item,
				};
				
				area.set_at(c_x, c_y, item_to_place.opt());
				self.remove_cursor();
			},
			// Case 2: item only in inventory
			(None, Some(mut item)) => {
				match mode {
					ClickMode::Left => {
						if !self.get_area_mut(target_area).add_item(&mut item) {
							// If target is full, try to add anywhere
							self.add_item_anywhere(&mut item);
						}
					},
					ClickMode::Right => {
						let half_stack = item.split_stack();
						area.set_at(c_x, c_y, item.opt());
						let Some(mut item) = half_stack else { return };
						if !self.get_area_mut(target_area).add_item(&mut item) {
							// If target is full, try to add anywhere
							self.add_item_anywhere(&mut item);
						}
					},
					ClickMode::Middle => {
						area.set_at(c_x, c_y, item.clone().opt());
						// Don't remove from area for middle-click (creative mode behavior)
						item.set_to_max_stack();
						if !self.get_area_mut(target_area).add_item(&mut item) {
							// If target is full, try to add anywhere
							self.add_item_anywhere(&mut item);
						}
					},
				}
			},
			// Case 3: both have item -> do case 2
			(Some(_cursor_item), Some(mut item)) => {
				match mode {
					ClickMode::Left => {
						if !self.get_area_mut(target_area).add_item(&mut item) {
							// If target is full, try to add anywhere
							self.add_item_anywhere(&mut item);
						}
					},
					ClickMode::Right => {
						let half_stack = item.split_stack();
						area.set_at(c_x, c_y, item.opt());
						let Some(mut item) = half_stack else { return };
						if !self.get_area_mut(target_area).add_item(&mut item) {
							// If target is full, try to add anywhere
							self.add_item_anywhere(&mut item);
						}
					},
					ClickMode::Middle => {
						area.set_at(c_x, c_y, item.clone().opt());
						// Don't remove from area for middle-click (creative mode behavior)
						item.set_to_max_stack();
						if !self.get_area_mut(target_area).add_item(&mut item) {
							// If target is full, try to add anywhere
							self.add_item_anywhere(&mut item);
						}
					},
				}
			},
			// Case 4: Both empty - do nothing
			(None, None) => {},
		}
	}

	/// Handles normal (non-shift) clicks
	fn handle_normal_click(&mut self, cursor: Option<ItemStack>, c_x: u8, c_y: u8, area_type: AreaType, mode: ClickMode) {
		let area = self.get_area_mut(area_type);
		
		// Check armor restrictions
		if area_type == AreaType::Armor {
			if let Some(ref cursor_item) = cursor {
				if !cursor_item.is_armor() {
					return;
				}
			}
		}
		
		match (cursor, area.remove_at(c_x, c_y)) {
			// Case 1: Place item from cursor into empty slot
			(Some(cursor_item), None) => {
				let item_to_place = match mode {
					ClickMode::Right => {
						let single_item = cursor_item.clone().with_stack_size(1);
						let remaining = cursor_item.remove_from_stack(1);
						area.set_at(c_x, c_y, single_item.opt());
						self.set_cursor(remaining);
						return;
					},
					ClickMode::Left | ClickMode::Middle => cursor_item,
				};
				
				area.set_at(c_x, c_y, item_to_place.opt());
				self.remove_cursor();
			},
			// Case 2: Pick up item with empty cursor
			(None, Some(mut item)) => {
				match mode {
					ClickMode::Left => {
						self.set_cursor(item.opt());
					},
					ClickMode::Right => {
						let half_stack = item.split_stack();
						area.set_at(c_x, c_y, item.opt());
						self.set_cursor(half_stack);
					},
					ClickMode::Middle => {
						area.set_at(c_x, c_y, item.clone().opt());
						// Don't remove from area for middle-click (creative mode behavior)
						item.set_to_max_stack();
						self.set_cursor(item.opt());
					},
				}
			},
			// Case 3: Interact with both cursor and slot having items
			(Some(cursor_item), Some(mut item)) => {
				if !cursor_item.can_stack_with(&item) {
					// Items don't stack - swap them
					area.set_at(c_x, c_y, cursor_item.opt());
					self.set_cursor(item.opt());
				} else {
					// Items can stack
					match mode {
						ClickMode::Left => {
							let remaining = item.add_to_stack(cursor_item.stack());
							area.set_at(c_x, c_y, item.opt());
							self.set_cursor(cursor_item.with_stack_size(remaining).opt());
						},
						ClickMode::Right => {
							let remaining = item.add_to_stack(1);
							area.set_at(c_x, c_y, item.opt());
							let new_cursor = cursor_item.remove_from_stack(1 - remaining);
							self.set_cursor(new_cursor);
						},
						ClickMode::Middle => {
							// Swap items for middle-click
							area.set_at(c_x, c_y, cursor_item.opt());
							self.set_cursor(item.opt());
						},
					}
				}
			},
			// Case 4: Both empty - do nothing
			(None, None) => {},
		}
	}

	/// Helper to consume crafting materials after taking output
	fn consume_crafting_materials(&mut self, count: u32) {
		let input_area = self.get_area_mut(AreaType::Storage);
		for item_slot in input_area.iter_mut() {
			let Some(item) = item_slot else { continue };

			*item_slot = item.clone().remove_from_stack(count);
		}
	}
}

