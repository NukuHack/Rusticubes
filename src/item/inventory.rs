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

// Default inventory sizes - can be upgraded during gameplay
pub const DEFAULT_ARMOR_SLOTS: u8 = 4;
pub const DEFAULT_HOTBAR_SLOTS: u8 = 5;
pub const DEFAULT_INV_ROWS: u8 = 3;
pub const DEFAULT_INV_COLS: u8 = 7;

/// A unified container for items that can handle both 1D and 2D layouts
#[derive(Clone, PartialEq)]
pub struct ItemContainer {
	items: Vec<Option<ItemStack>>,
	size: Slot,
}

impl ItemContainer {
	/// Create a new 2D grid container (like main inventory)
	pub fn new(rows: u8, cols: u8) -> Self {
		Self::with_dimensions(Slot::custom(rows, cols))
	}
	pub fn with_dimensions(size: Slot) -> Self {
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
	fn calculate_index(&self, row: u8, col: u8) -> Option<usize> {
		if self.is_linear() {
			// For linear containers, use whichever coordinate is non-zero
			return Some((row + col) as usize);
		}

		if row >= self.rows() || col >= self.cols() {
			return None;
		}

		Some((row as usize * self.cols() as usize) + col as usize)
	}

	/// Returns an iterator over all slots with their indices and items
	#[inline] 
	pub fn slot_iter(&self) -> impl Iterator<Item = (usize, &Option<ItemStack>)> {
		self.items
			.iter()
			.enumerate()
			.take(self.capacity())
	}
	
	/// Returns an iterator over all slots with their indices and items
	#[inline] 
	pub fn slot_iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut Option<ItemStack>)> {
		let cap = self.capacity();
		self.items
			.iter_mut()
			.enumerate()
			.take(cap)
	}

	/// Uses the slot iterator to set items based on a predicate
	/// Returns the number of items set
	pub fn update_items<F>(&mut self, mut predicate: F) -> usize
	where
		F: FnMut(usize, &Option<ItemStack>) -> Option<ItemStack>,
	{
		let mut count = 0;
		for (index, slot) in self.slot_iter_mut() {
			if let Some(new_item) = predicate(index, slot) {
				*slot = Some(new_item);
				count += 1;
			}
		}
		count
	}

	/// Find the first empty slot
	#[inline] 
	pub fn find_empty_slot(&self) -> Option<usize> {
		self.items.iter().position(|slot| slot.is_none())
	}

	/// Count non-empty slots
	#[inline] 
	pub fn count_items(&self) -> usize {
		self.items.iter().filter(|slot| slot.is_some()).count()
	}

	/// Check if the container is full
	#[inline] 
	pub fn is_full(&self) -> bool {
		self.find_empty_slot().is_none()
	}

	/// Add an item to the first available slot
	#[inline] pub fn add_item(&mut self, mut new_item: ItemStack) -> bool {
	    // First try to stack with existing items of the same type
	    for (_, slot) in self.slot_iter_mut() {
	        if let Some(existing_item) = slot {
	            if existing_item.can_stack_with(&new_item) {
	                let remaining = existing_item.add_quantity(new_item.quantity);
	                if remaining == 0 {
	                    // Fully stacked the new item
	                    return true;
	                }
	                // Partially stacked, continue with remaining quantity
	                new_item.set_quantity(remaining);
	            }
	        }
	    }

	    // If we still have items left, try to find an empty slot
	    if let Some(index) = self.find_empty_slot() {
	        self.set(index, Some(new_item));
	        true
	    } else {
	        false
	    }
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
/*
	/// Swap items between two linear indices
	pub fn swap(&mut self, a: usize, b: usize) -> bool {
		if a >= self.capacity() || b >= self.capacity() {
			return false;
		}
		self.items.swap(a, b);
		true
	}

	/// Swap items between two grid positions
	pub fn swap_at(&mut self, row_a: u8, col_a: u8, row_b: u8, col_b: u8) -> bool {
		if self.is_1d() {
			// For 1D containers, use whichever coordinate is non-zero
			let index_a = (row_a+col_a) as usize;
			let index_b = (row_b+col_b) as usize;
			return self.swap(index_a as usize, index_b as usize);
		}
		
		if row_a >= self.rows || col_a >= self.cols || row_b >= self.rows || col_b >= self.cols {
			return false;
		}
		let index_a = (row_a as usize * self.cols() as usize) + col_a as usize;
		let index_b = (row_b as usize * self.cols() as usize) + col_b as usize;
		self.swap(index_a, index_b)
	}
*/
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
					if capacity % i == 0 {
						let j = capacity / i;
						let diff = (i as i32 - j as i32).abs();
						if diff < min_diff {
							min_diff = diff;
							best = (i, j);
						}
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
	
	/// Get the currently selected hotbar item
	#[inline] pub fn selected_item(&self) -> Option<&ItemStack> {
		self.hotbar.get(self.selected_index() as usize)
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

	/// Get item at specific inventory position
	#[inline] pub fn get_item(&self, row: u8, col: u8) -> Option<&ItemStack> {
		self.items.get_at(row, col)
	}
	/// Get armor item at specific slot
	#[inline] pub fn get_armor(&self, slot: u8) -> Option<&ItemStack> {
		self.armor.get(slot as usize)
	}
	/// Get hotbar item at specific slot
	#[inline] pub fn get_hotbar(&self, slot: u8) -> Option<&ItemStack> {
		self.hotbar.get(slot as usize)
	}
	
	// Inventory expansion
	pub fn expand(&mut self, rows: u8, cols: u8, hotbar_slots: u8, armor_slots: u8) {
		self.items.resize(rows, cols);
		self.hotbar.expand(hotbar_slots);
		self.armor.expand(armor_slots);
		self.select_slot(self.selected_slot as isize);
	}
	
	/// Set the UI layout
	#[inline] pub fn set_layout(&mut self, layout: &InventoryLayout) {
		self.layout = Some(layout.clone());
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
	#[inline] pub fn get_area_mut(&mut self, area: &AreaType) -> &mut ItemContainer {
		match area {
			AreaType::Inventory => &mut self.items,
			AreaType::Hotbar => &mut self.hotbar,
			AreaType::Armor => &mut self.armor,
			_ => &mut self.items, // should return nothing
		}
	}

	/// Add item to any available slot (tries hotbar first, then inventory, then armor)
	#[inline] pub fn add_item_anywhere(&mut self, item: ItemStack) -> bool {
		self.hotbar.add_item(item.clone()) ||
		self.items.add_item(item.clone()) ||
		self.armor.add_item(item)
	}
	/// Count total items across all containers
	#[inline] pub fn total_count(&self) -> usize {
		self.armor.count_items() + self.hotbar.count_items() + self.items.count_items()
	}
	#[inline] pub fn is_full(&self) -> bool {
		self.armor.is_full() && self.hotbar.is_full() && self.items.is_full()
	}
}