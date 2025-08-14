
use crate::item::ui_inventory::InventoryLayout;
use crate::item::items::ItemStack;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AreaType { 
	Panel,  Inventory,  Hotbar, 
	Armor,  Storage,  Input,  Output 
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

	/// Resize the container to match its capacity
	#[inline] 
	fn resize_to_capacity(&mut self) {
		self.items.resize(self.capacity(), None);
	}
	/// Get dimensions
	#[inline] pub const fn rows(&self) -> u8 { self.size.rows() }
	#[inline] pub const fn cols(&self) -> u8 { self.size.cols() }
	#[inline] pub const fn capacity(&self) -> usize { self.rows() as usize * self.cols() as usize }

	/// Check if this is a 1D container (single row or single column)
	#[inline] pub const fn is_linear(&self) -> bool {
		self.rows() == 1 || self.cols() == 1
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
	#[inline] fn calculate_index(&self, row: u8, col: u8) -> Option<usize> {
		if self.is_linear() {
			// For linear containers, use whichever coordinate is non-zero
			let sum = row as usize + col as usize;
			if sum >= self.capacity() { return None; }

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

	/// Returns an iterator over all slots with their indices and items
	#[inline] pub fn slot_iter(&self) -> impl Iterator<Item = (usize, &Option<ItemStack>)> {
		self.items.iter().enumerate().take(self.capacity())
	}
	
	/// Returns an iterator over all slots with their indices and items
	#[inline] pub fn slot_iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut Option<ItemStack>)> {
		let cap = self.capacity();
		self.items.iter_mut().enumerate().take(cap)
	}

	/// Uses the slot iterator to set items based on a predicate
	/// Returns the number of items set
	#[inline] pub fn update_items<F>(&mut self, mut predicate: F) -> usize
	where
		F: FnMut(usize, &Option<ItemStack>) -> Option<ItemStack>,
	{
		let mut count = 0;
		for (index, slot) in self.slot_iter_mut() {
			let Some(new_item) = predicate(index, slot) else { continue; };

			*slot = Some(new_item);
			count += 1;
		}
		count
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

	/// Add an item to the first available slot
	#[inline] pub fn add_item(&mut self, mut new_item: ItemStack) -> bool {
		// First try to stack with existing items of the same type
		for (_, slot) in self.slot_iter_mut() {
			let Some(existing_item) = slot else { continue; };

			if !existing_item.can_stack_with(&new_item) { continue; }

			let remaining = existing_item.add_to_stack(new_item.stack);
			if remaining == 0 { return true; }// Fully stacked the new item

			// Partially stacked, continue with remaining stack
			new_item.set_stack_size(remaining);
		}

		// If we still have items left, try to find an empty slot
		let Some(index) = self.find_empty_slot() else { return false; };

		self.set(index, Some(new_item));
		return true;
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
		self.resize_with_dimensions(Slot::custom(rows, cols));
	}
	#[inline] pub fn resize_with_dimensions(&mut self, size: Slot) {
		self.size = size;
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

	// Bulk operations
	#[inline] pub fn items(&self) -> &[Option<ItemStack>] {
		&self.items
	}
	#[inline] pub fn clear(&mut self) {
		for slot in &mut self.items {
			*slot = None;
		}
	}
	#[inline] pub fn iter(&self) -> impl Iterator<Item = &Option<ItemStack>> {
		self.items.iter()
	}
	#[inline] pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Option<ItemStack>> {
		self.items.iter_mut()
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
			cursor_item: None,
			layout: None,
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
	#[inline] pub fn select_slot(&mut self, idx: isize) {
		self.selected_slot = match idx {
			i if i >= self.hotbar.capacity() as isize => 0,
			i if i < 0 => self.hotbar.capacity() - 1,
			i => i as usize,
		};
	}
	#[inline] pub fn step_select_slot(&mut self, delta: f32) {
		let step = if delta > 0.0 { 1 } else if delta < 0.0 { -1 } else { 0 };
		self.select_slot(self.selected_slot as isize + step);
	}

	#[inline] pub fn get_cursor(&self) -> Option<&ItemStack> {
		self.cursor_item.as_ref()
	}
	#[inline] pub fn set_cursor(&mut self, item: Option<ItemStack>) {
		self.cursor_item = item;
	}
	#[inline] pub fn remove_cursor(&mut self) -> Option<ItemStack> {
		self.cursor_item.take()
	}
	
	/// Set the UI layout
	#[inline] pub fn set_layout(&mut self, layout: &InventoryLayout) {
		self.layout = Some(layout.clone());
	}
	#[inline] pub fn get_layout(&self) -> Option<&InventoryLayout> {
		self.layout.as_ref()
	}
	
	/// Get total item capacity
	#[inline] pub fn total_capacity(&self) -> usize {
		self.armor.capacity() + self.hotbar.capacity() + self.items.capacity()
	}

	#[inline] pub fn get_area(&self, area: &AreaType) -> &ItemContainer {
		match area {
			AreaType::Inventory => &self.items,
			AreaType::Hotbar => &self.hotbar,
			AreaType::Armor => &self.armor,
			_ => &self.items, // should return nothing
		}
	}
	#[inline] pub fn try_get_area_mut(&mut self, area: AreaType) -> Option<&mut ItemContainer> {
		match area {
			AreaType::Inventory => Some(&mut self.items),
			AreaType::Hotbar => Some(&mut self.hotbar),
			AreaType::Armor => Some(&mut self.armor),
			_ => None, // should return nothing
		}
	}
	#[inline] pub fn get_area_mut(&mut self, area: AreaType) -> &mut ItemContainer {
		self.try_get_area_mut(area).unwrap_or_else(|| panic!("Invalid area type: {:?}", area)) // temporary will make the others work
	}

	/// Add item to any available slot (tries hotbar first, then inventory, then armor - if is an armor item)
	#[inline] pub fn add_item_anywhere(&mut self, item: ItemStack) -> bool {
		self.hotbar.add_item(item.clone()) ||
		self.items.add_item(item.clone()) ||
		(item.is_armor() && self.armor.add_item(item))
	}
	/// Count total items across all containers
	#[inline] pub fn total_count(&self) -> usize {
		self.armor.count_items() + self.hotbar.count_items() + self.items.count_items()
	}
	#[inline] pub fn is_full(&self) -> bool {
		self.armor.is_full() && self.hotbar.is_full() && self.items.is_full()
	}


	pub fn handle_click_press(&mut self, clicked_pos:(u8,u8), area_type: AreaType) {
		if area_type == AreaType::Storage || area_type == AreaType::Input || area_type == AreaType::Output { return; } // hotfix for the crash made by these if user clicks
		 // because we can not get area mutably for these tipes

		let cursor = self.get_cursor().cloned();
		let area = self.get_area_mut(area_type); let (c_x, c_y) = clicked_pos;
		let armor_in_not_armor_area = area_type == AreaType::Armor && !cursor.clone().map(|item| item.is_armor()).unwrap_or(false);

		match (cursor, area.remove_at(c_x, c_y).clone()) {
			// Case 1: Trying to place an item from cursor
			(Some(cursor_item), None) => {
				if armor_in_not_armor_area { return; }
				area.set_at(c_x, c_y, cursor_item.opt());
				self.remove_cursor();
			},
			// Case 2: Trying to pick up an item with empty cursor
			(None, Some(item)) => {
				area.remove_at(c_x, c_y);
				self.set_cursor(item.opt());
			},
			// Case 3: Trying to place an item from cursor into an item
			(Some(cursor_item), Some(mut item)) => {
				if armor_in_not_armor_area { return; }
				if !item.can_stack_with(&cursor_item) { // if can't stack switch
					area.set_at(c_x, c_y, cursor_item.opt());
					self.set_cursor(item.opt());
					return;
				}
				// else : they can stack -> they are the same type
				// let the inventory have the "main" item and the cursor will have the remaining
				let rem = item.add_to_stack(cursor_item.stack);
				area.set_at(c_x, c_y, item.clone().opt());
				self.set_cursor(item.with_stack_size(rem).opt());
			},
			// Case 4: Both empty
			(None, None) => {}
		}
	}
	pub fn handle_rclick_press(&mut self, clicked_pos:(u8,u8), area_type: AreaType) {
		if area_type == AreaType::Storage || area_type == AreaType::Input || area_type == AreaType::Output { return; } // hotfix for the crash made by these if user clicks
		 // because we can not get area mutably for these tipes

		let cursor = self.get_cursor().cloned();
		let area = self.get_area_mut(area_type); let (c_x, c_y) = clicked_pos;
		let armor_in_not_armor_area = area_type == AreaType::Armor && !cursor.clone().map(|item| item.is_armor()).unwrap_or(false);

		match (cursor, area.remove_at(c_x, c_y).clone()) {
			// Case 1: Trying to place an item from cursor
			(Some(cursor_item), None) => {
				if armor_in_not_armor_area { return; }
				area.set_at(c_x, c_y, cursor_item.clone().with_stack_size(1).opt());
				self.set_cursor(cursor_item.remove_from_stack(1));
			},
			// Case 2: Trying to pick up half the item with empty cursor
			(None, Some(mut item)) => {
				// this is the smaller if the number is odd
				let half_stack = item.split_stack();
				area.set_at(c_x, c_y, item.opt());
				// the "bigger half" will be on the cursor and the smaller side will be left in the inventory
				self.set_cursor(half_stack);
				
			},
			// Case 3: Trying to place an item from cursor into an item
			(Some(cursor_item), Some(mut item)) => {
				if armor_in_not_armor_area { return; }
				if !item.can_stack_with(&cursor_item) { // if can't stack switch
					area.set_at(c_x, c_y, cursor_item.opt());
					self.set_cursor(item.opt());
					return;
				}
				// else : they can stack -> they are the same type
				// let the cursor have the "main" item and the inventory will have the remaining
				let rem = item.add_to_stack(cursor_item.stack);
				area.set_at(c_x, c_y, item.clone().with_stack_size(rem).opt());
				self.set_cursor(item.opt());
			},
			// Case 4: Both empty
			(None, None) => {}
		}
	}
	pub fn handle_mclick_press(&mut self, clicked_pos:(u8,u8), area_type: AreaType) {
		let cursor = self.get_cursor().cloned(); let (c_x, c_y) = clicked_pos;
		let item = if area_type == AreaType::Storage || area_type == AreaType::Input || area_type == AreaType::Output {
			let main_item = ItemStack::new(ItemStack::lut_by_index((c_x + 1 * c_y) as usize).name.to_string()); // TODO : MAKE THIS ACTUALLY WORK AND NOT JUST A BASIC SOLUTION
			main_item.opt()
		} else { self.get_area_mut(area_type).get_at(c_x, c_y).cloned() };
		let armor_in_not_armor_area = area_type == AreaType::Armor && !cursor.clone().map(|item| item.is_armor()).unwrap_or(false);

		match (cursor, item) {
			// Case 1: 
			(Some(_cursor_item), None) => {
				// nothign happens
			},
			// Case 2: Clicked on item with empty cursor
			(None, Some(mut item)) => {
				item.set_to_max_stack();
				self.set_cursor(Some(item));
				
			},
			// Case 3: Both full, Switch
			(Some(cursor_item), Some(item)) => {
				if armor_in_not_armor_area { return; }
				self.set_cursor(item.opt());
				if area_type == AreaType::Storage || area_type == AreaType::Input || area_type == AreaType::Output { return; }
				// because we can not get area mutably for these tipes
				self.get_area_mut(area_type).set_at(c_x, c_y, cursor_item.opt());
			},
			// Case 4: Both empty
			(None, None) => {}
		}
	}
}
