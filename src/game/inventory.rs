use crate::ui::inventory::{AreaType, InventoryLayout};
use crate::game::items::ItemStack;

// Default inventory sizes - can be upgraded during gameplay
pub const DEFAULT_ARMOR_SLOTS: u8 = 4;
pub const DEFAULT_HOTBAR_SLOTS: u8 = 5;
pub const DEFAULT_INV_ROWS: u8 = 3;
pub const DEFAULT_INV_COLS: u8 = 7;

/// A unified container for items that can handle both 1D and 2D layouts
#[derive(Clone, PartialEq)]
pub struct ItemContainer {
    items: Vec<Option<ItemStack>>,
    rows: u8,
    cols: u8,
}

impl ItemContainer {
    /// Create a new 2D grid container (like main inventory)
    pub fn new(rows: u8, cols: u8) -> Self {
        let mut container = Self {
            items: Vec::new(),
            rows,
            cols,
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
    #[inline] 
    pub const fn rows(&self) -> u8 { self.rows }
    #[inline] 
    pub const fn cols(&self) -> u8 { self.cols }
    #[inline] 
    pub const fn capacity(&self) -> usize { self.rows() as usize * self.cols() as usize }

    /// Check if this is a 1D container (single row or single column)
    #[inline]
    pub const fn is_1d(&self) -> bool {
        self.rows() == 1 || self.cols() == 1
    }

    /// Get an item by linear index
    #[inline] 
    pub fn get(&self, index: usize) -> Option<&ItemStack> {
        if index >= self.capacity() {
            return None;
        }
        self.items.get(index)?.as_ref()
    }

    /// Get item at grid position (row, col) - works for both 1D and 2D
    #[inline] 
    pub fn get_at(&self, row: u8, col: u8) -> Option<&ItemStack> {
        if self.is_1d() {
            // For 1D containers, use whichever coordinate is non-zero
            return self.get((row+col) as usize);
        }
        
        if row >= self.rows() || col >= self.cols() {
            return None;
        }
        let index = (row as usize * self.cols() as usize) + col as usize;
        self.get(index)
    }

    /// Set an item by linear index
    #[inline] 
    pub fn set(&mut self, index: usize, item: Option<ItemStack>) -> bool {
        if index >= self.capacity() {
            return false;
        }
        if let Some(slot) = self.items.get_mut(index) {
            *slot = item;
            true
        } else {
            false
        }
    }

    /// Set item at grid position (row, col)
    #[inline] 
    pub fn set_at(&mut self, row: u8, col: u8, item: Option<ItemStack>) -> bool {
        if self.is_1d() {
            // For 1D containers, use whichever coordinate is non-zero
            return self.set((row+col) as usize, item);
        }
        
        if row >= self.rows() || col >= self.cols() {
            return false;
        }
        let index = (row as usize * self.cols() as usize) + col as usize;
        self.set(index, item)
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
    pub fn set_items_with_iterator<F>(&mut self, mut predicate: F) -> usize
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
    #[inline] 
    pub fn add_item(&mut self, item: ItemStack) -> bool {
        if let Some(index) = self.find_empty_slot() {
            self.set(index, Some(item));
            true
        } else {
            false
        }
    }

    /// Remove an item at the specified linear index
    #[inline] 
    pub fn remove(&mut self, index: usize) -> Option<ItemStack> {
        if index >= self.capacity() {
            return None;
        }
        self.items.get_mut(index)?.take()
    }

    /// Remove an item at grid position (row, col)
    #[inline] 
    pub fn remove_at(&mut self, row: u8, col: u8) -> Option<ItemStack> {
        if self.is_1d() {
            // For 1D containers, use whichever coordinate is non-zero
            return self.remove((row+col) as usize);
        }
        
        if row >= self.rows || col >= self.cols {
            return None;
        }
        let index = (row as usize * self.cols() as usize) + col as usize;
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
    #[inline] 
    pub fn upgrade_dimensions(&mut self, new_rows: u8, new_cols: u8) {
        self.rows = new_rows;
        self.cols = new_cols;
        self.resize_to_capacity();
    }

    /// Upgrade capacity by finding appropriate dimensions (works for both linear and 2D containers)
    #[inline] 
    pub fn upgrade_capacity(&mut self, new_capacity: u8) {
        if new_capacity == 0 {
            self.upgrade_dimensions(0, 0);
            return;
        }

        // For linear containers, keep them linear
        if self.is_1d() && self.capacity() < 15 {
            if self.rows() == 1 {
                self.upgrade_dimensions(1, new_capacity);
            } else {
                self.upgrade_dimensions(new_capacity, 1);
            }
            return;
        }

        // For 2D containers, find the most square-like dimensions possible
        let (new_rows, new_cols) = Self::find_best_dimensions(new_capacity);
        self.upgrade_dimensions(new_rows, new_cols);
    }

    /// Find the most balanced dimensions for a given capacity
    fn find_best_dimensions(capacity: u8) -> (u8, u8) {
        if capacity <= 4 {
            // Small capacities work best as single row
            return (1, capacity);
        }

        let mut best_pair = (1, capacity);
        let mut best_diff = capacity as i32 - 1;

        // Find the factor pair with the smallest difference between them
        for i in 1..=(capacity as f32).sqrt().ceil() as u8 {
            if capacity % i == 0 {
                let j = capacity / i;
                let diff = (i as i32 - j as i32).abs();
                if diff < best_diff {
                    best_diff = diff;
                    best_pair = (i, j);
                }
            }
        }

        // Prefer wider containers (more columns than rows) as they're more common in UIs
        if best_pair.0 > best_pair.1 {
            (best_pair.1, best_pair.0)
        } else {
            best_pair
        }
    }

    /// Get all items as a vector (for compatibility)
    #[inline] 
    pub fn get_all_items(&self) -> Vec<Option<ItemStack>> {
        self.items.clone()
    }

    /// Clear all items from the container
    #[inline] 
    pub fn clear(&mut self) {
        for slot in &mut self.items {
            *slot = None;
        }
    }

    /// Iterator over all items
    #[inline] 
    pub fn iter(&self) -> impl Iterator<Item = &Option<ItemStack>> {
        self.items.iter()
    }

    /// Mutable iterator over all items
    #[inline] 
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Option<ItemStack>> {
        self.items.iter_mut()
    }
}

#[derive(Clone, PartialEq)]
pub struct Inventory {
	selected_slot_idx: usize,
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
			selected_slot_idx: 0,
			armor: ItemContainer::new(1, armor_slots),
			items: ItemContainer::new(rows, cols),
			hotbar: ItemContainer::new(1, hotbar_slots),
			cursor_item: None,
			layout: None,
		}
	}
	
	// Getters for compatibility
	#[inline] pub const fn armor_capacity(&self) -> usize { self.armor.capacity() }
	#[inline] pub const fn hotbar_capacity(&self) -> usize { self.hotbar.capacity() }
	#[inline] pub const fn inv_row(&self) -> u8 { self.items.rows() }
	#[inline] pub const fn inv_col(&self) -> u8 { self.items.cols() }
	#[inline] pub const fn inv_capacity(&self) -> usize { self.items.capacity() }
	#[inline] pub const fn ssi(&self) -> usize { self.selected_slot_idx }
	
	/// Get the currently selected hotbar item
	#[inline] pub fn selected_item(&self) -> Option<&ItemStack> {
		self.hotbar.get(self.ssi() as usize)
	}
	
	/// Select a different hotbar slot
	#[inline] pub fn select_slot(&mut self, idx: isize) {
		self.selected_slot_idx = if idx >= self.hotbar_capacity() as isize {
			0 // first item
		} else if idx < 0 {
			self.hotbar_capacity() - 1 // last item
		} else {
			idx as usize // just the input index
		};
	}
	#[inline] pub fn step_select_slot(&mut self, delta: f32) {
		let way:isize = if delta > 0. { 1 } else if delta < 0. { -1 } else { 0 };
		self.select_slot(self.ssi() as isize + way);
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
	
	/// Upgrade inventory capacity (for game progression)
	pub fn upgrade_capacity(&mut self, new_rows: u8, new_cols: u8, new_hotbar: u8, new_armor: u8) {
		self.items.upgrade_dimensions(new_rows, new_cols);
		self.hotbar.upgrade_capacity(new_hotbar);
		self.armor.upgrade_capacity(new_armor);
		
		// Clamp selected slot if it's now out of bounds
		self.select_slot(self.ssi() as isize);
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
	#[inline] pub fn count_all_items(&self) -> usize {
		self.armor.count_items() + self.hotbar.count_items() + self.items.count_items()
	}

	/// Check if entire inventory is full
	#[inline] pub fn is_completely_full(&self) -> bool {
		self.armor.is_full() && self.hotbar.is_full() && self.items.is_full()
	}
}