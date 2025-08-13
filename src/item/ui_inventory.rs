
use glam::Vec2;
use crate::item::inventory::{Inventory, ItemContainer, AreaType, Slot};
use crate::ext::ptr;
use crate::ui::{manager::{UIManager, UIState, FocusState}, element::UIElement};
use crate::item::items::ItemStack;
use crate::utils::color::Solor;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum InventoryUIState {
	Player { inv: InvState },
	Storage { inv: InvState, size: Slot },
	Crafting { inv: InvState, size: Slot, result: Slot },
}

impl InventoryUIState {
	#[inline] pub const fn pl() -> PlayerInvBuilder { PlayerInvBuilder::new() }
	#[inline] pub const fn str() -> StorageInvBuilder { StorageInvBuilder::new() }
	#[inline] pub const fn craft() -> CraftingInvBuilder { CraftingInvBuilder::new() }
	#[inline] pub const fn default() -> Self { Self::Player { inv: InvState::All } }
}

macro_rules! builder {
	($name:ident { $($field:ident: $ty:ty = $default:expr),+ }) => {
		pub struct $name { $(pub $field: $ty),+ }
		impl $name {
			#[inline] const fn new() -> Self { Self { $($field: $default),+ } }
		}
	};
}

builder!(PlayerInvBuilder { inv: InvState = InvState::Items });
impl PlayerInvBuilder {
	#[inline] pub const fn inv(mut self, inv: InvState) -> Self { self.inv = inv; self }
	#[inline] pub const fn b(self) -> InventoryUIState { InventoryUIState::Player { inv: self.inv } }
}

builder!(StorageInvBuilder { inv: InvState = InvState::Items, size: Slot = Slot::MED });
impl StorageInvBuilder {
	#[inline] pub const fn inv(mut self, inv: InvState) -> Self { self.inv = inv; self }
	#[inline] pub const fn size(mut self, size: Slot) -> Self { self.size = size; self }
	#[inline] pub const fn b(self) -> InventoryUIState {
		InventoryUIState::Storage { inv: self.inv, size: self.size }
	}
}

builder!(CraftingInvBuilder { inv: InvState = InvState::Items, size: Slot = Slot::custom(3, 3), result: Slot = Slot::custom(1, 1) });
impl CraftingInvBuilder {
	#[inline] pub const fn inv(mut self, inv: InvState) -> Self { self.inv = inv; self }
	#[inline] pub const fn input(mut self, size: Slot) -> Self { self.size = size; self }
	#[inline] pub const fn result(mut self, result: Slot) -> Self { self.result = result; self }
	#[inline] pub const fn b(self) -> InventoryUIState {
		InventoryUIState::Crafting { inv: self.inv, size: self.size, result: self.result }
	}
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum InvState { All, Items, Armor, Inventory, Hotbar, None }

const SLOT: f32 = 0.08;
const PADDING: f32 = 0.02;

#[derive(Clone, PartialEq, Debug)]
pub struct InventoryLayout {
	pub section_spacing: f32,
	pub panel_padding: f32,
	pub panel_position: Vec2, // bottom-left corner
	pub panel_size: Vec2,
	
	// Calculated positions for different sections
	pub areas: Vec<AreaLayout>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AreaLayout {
	pub name: AreaType,
	pub position: (f32, f32), // bottom-left corner of the area
	pub size: (f32, f32),
	pub rows: u8,
	pub columns: u8,
}

impl AreaLayout {
	#[inline] pub const fn new(rows: u8, columns: u8, center_pos: (f32, f32), name: AreaType) -> Self {
		let w = (columns as f32 * SLOT) + ((columns.saturating_sub(1)) as f32 * PADDING);
		let h = (rows as f32 * SLOT) + ((rows.saturating_sub(1)) as f32 * PADDING);
		Self {
			name, rows, columns,
			position: (center_pos.0 - w / 2.0, center_pos.1 - h / 2.0),
			size: (w, h),
		}
	}
	
	#[inline] pub const fn get_bounds(&self) -> (f32, f32, f32, f32) {
		(self.position.0, self.position.1, self.position.0 + self.size.0, self.position.1 + self.size.1)
	}
	
	#[inline] pub const fn contains_point(&self, x: f32, y: f32) -> bool {
		let (min_x, min_y, max_x, max_y) = self.get_bounds();
		x >= min_x && x <= max_x && y >= min_y && y <= max_y
	}
	
	#[inline] pub const fn get_slot_position(&self, row: u8, col: u8) -> (f32, f32) {
		if row >= self.rows || col >= self.columns { 
			return (self.position.0, self.position.1); // Return area origin as fallback
		}
		(
			self.position.0 + (col as f32 * (SLOT + PADDING)),
			self.position.1 + ((self.rows - 1 - row) as f32 * (SLOT + PADDING))
		)
	}
	
	#[inline] pub const fn slot_contains_point(&self, row: u8, col: u8, x: f32, y: f32) -> bool {
		let (sx, sy) = self.get_slot_position(row, col);
		x >= sx && x <= sx + SLOT && y >= sy && y <= sy + SLOT
	}
}

impl InventoryLayout {
	#[inline] 
	pub fn calculate_for_player(inv_state: InvState, inv: &mut Inventory) -> Self {
		fn calculate_player_positions(
			inv_area: &(u8, u8), hotbar_area: &(u8, u8), armor_area: &(u8, u8), spacing: f32
		) -> ((f32, f32), (f32, f32), (f32, f32)) {
			let inv_size = InventoryLayout::area_size(inv_area);
			let hotbar_size = InventoryLayout::area_size(hotbar_area);
			let armor_size = InventoryLayout::area_size(armor_area);
			
			let inv_center = (0.0, 0.0);
			let hotbar_center = (0.0, -inv_size.1/2.0 - spacing - hotbar_size.1/2.0);
			let armor_center = (-inv_size.0/2.0 - spacing - armor_size.0/2.0, 0.0);
			
			(inv_center, hotbar_center, armor_center)
		}
		let inv_layout = &ptr::get_settings().inv_layout;
		let mut layout = Self::default();
		let inv = inv.clone();
		
		// Special case for hotbar only
		let hot_siz = inv.hotbar_size();
		if inv_state == InvState::Hotbar {
			layout.areas.push(AreaLayout::new(hot_siz.0, hot_siz.1, inv_layout.hotbar, AreaType::Hotbar));
			layout.finalize_layout(inv_state, &[]);
			return layout;
		}
		
		let (inv_area, hotbar_area, armor_area) = Self::create_player_areas(&inv);
		let positions = calculate_player_positions(&inv_area, &hotbar_area, &armor_area, layout.section_spacing);
		
		layout.add_areas(&[
			(inv_area, positions.0, AreaType::Inventory),
			(hotbar_area, positions.1, AreaType::Hotbar),
			(armor_area, positions.2, AreaType::Armor),
		]);
		
		layout.finalize_layout(inv_state, &[]);
		layout
	}
	
	#[inline] 
	pub fn calculate_for_storage(storage_size: Slot, inv_state: InvState, inv_lay: &mut Inventory) -> Self {
		fn calculate_storage_positions(
			storage_area: &(u8, u8), inv_area: &(u8, u8), hotbar_area: &(u8, u8), armor_area: &(u8, u8), spacing: f32
		) -> ((f32, f32), (f32, f32), (f32, f32), (f32, f32)) {
			let storage_size = InventoryLayout::area_size(storage_area);
			let inv_size = InventoryLayout::area_size(inv_area);
			let hotbar_size = InventoryLayout::area_size(hotbar_area);
			let armor_size = InventoryLayout::area_size(armor_area);
			
			let storage_center = (0.0, storage_size.1/2.0 + spacing);
			let inv_center = (0.0, -inv_size.1/2.0 - spacing);
			let hotbar_center = (0.0, inv_center.1 - inv_size.1/2.0 - spacing - hotbar_size.1/2.0);
			let armor_center = (-inv_size.0/2.0 - spacing - armor_size.0/2.0, inv_center.1);
			
			(storage_center, inv_center, hotbar_center, armor_center)
		}
		let mut layout = Self::default();
		let inv_lay = inv_lay.clone();
		
		let storage_area = (storage_size.rows(), storage_size.cols());
		let (inv_area, hotbar_area, armor_area) = Self::create_player_areas(&inv_lay);
		let positions = calculate_storage_positions(&storage_area, &inv_area, &hotbar_area, &armor_area, layout.section_spacing);
		
		layout.add_areas(&[
			(storage_area, positions.0, AreaType::Storage),
			(inv_area, positions.1, AreaType::Inventory),
			(hotbar_area, positions.2, AreaType::Hotbar),
			(armor_area, positions.3, AreaType::Armor),
		]);
		
		layout.finalize_layout(inv_state, &[AreaType::Storage]);
		layout
	}
	
	#[inline] 
	pub fn calculate_for_crafting(crafting_size: Slot, result_size: Slot, inv_state: InvState, inv_lay: &mut Inventory) -> Self {
		fn calculate_crafting_positions(
			input_area: &(u8, u8), output_area: &(u8, u8), inv_area: &(u8, u8), hotbar_area: &(u8, u8), armor_area: &(u8, u8), spacing: f32
		) -> ((f32, f32), (f32, f32), (f32, f32), (f32, f32), (f32, f32)) {
			let input_size = InventoryLayout::area_size(input_area);
			let output_size = InventoryLayout::area_size(output_area);
			let inv_size = InventoryLayout::area_size(inv_area);
			let hotbar_size = InventoryLayout::area_size(hotbar_area);
			let armor_size = InventoryLayout::area_size(armor_area);
			
			let crafting_y = spacing + (input_size.1/2.0).max(output_size.1/2.0);
			let horizontal_spacing = (input_size.0 + output_size.0)/2.0 + spacing;
			
			let input_center = (-horizontal_spacing/2.0, crafting_y);
			let output_center = (horizontal_spacing/2.0, crafting_y);
			let inv_center = (0.0, -inv_size.1/2.0 - spacing);
			let hotbar_center = (0.0, inv_center.1 - inv_size.1/2.0 - spacing - hotbar_size.1/2.0);
			let armor_center = (-inv_size.0/2.0 - spacing - armor_size.0/2.0, inv_center.1);
			
			(input_center, output_center, inv_center, hotbar_center, armor_center)
		}
		let mut layout = Self::default();
		let inv_lay = inv_lay.clone();
		
		let input_area = (crafting_size.rows(), crafting_size.cols());
		let output_area = (result_size.rows(), result_size.cols());
		let (inv_area, hotbar_area, armor_area) = Self::create_player_areas(&inv_lay);
		let positions = calculate_crafting_positions(&input_area, &output_area, &inv_area, &hotbar_area, &armor_area, layout.section_spacing);
		
		layout.add_areas(&[
			(input_area, positions.0, AreaType::Input),
			(output_area, positions.1, AreaType::Output),
			(inv_area, positions.2, AreaType::Inventory),
			(hotbar_area, positions.3, AreaType::Hotbar),
			(armor_area, positions.4, AreaType::Armor),
		]);
		
		layout.finalize_layout(inv_state, &[AreaType::Input, AreaType::Output]);
		layout
	}
	
	// Helper functions
	fn create_player_areas(inv_lay: &Inventory) -> ((u8, u8), (u8, u8), (u8, u8)) {
		(
			inv_lay.inv_size(),
			inv_lay.hotbar_size(),
			inv_lay.armor_size(),
		)
	}
	
	
	
	fn area_size(area: &(u8, u8)) -> (f32, f32) {
		let w = (area.1 as f32 * SLOT) + ((area.1.saturating_sub(1)) as f32 * PADDING);
		let h = (area.0 as f32 * SLOT) + ((area.0.saturating_sub(1)) as f32 * PADDING);
		(w, h)
	}
	
	fn add_areas(&mut self, areas: &[((u8, u8), (f32, f32), AreaType)]) {
		for &((rows, cols), pos, area_type) in areas {
			self.areas.push(AreaLayout::new(rows, cols, pos, area_type));
		}
	}
	
	fn finalize_layout(&mut self, inv_state: InvState, extra_types: &[AreaType]) {
		let mut areas = self.areas.iter()
			.filter(|a| extra_types.contains(&a.name))
			.cloned()
			.collect::<Vec<_>>();
		areas.extend(self.get_areas_for_inv_state(inv_state));
		self.calculate_panel_bounds(&areas);
	}


	#[inline] pub const fn default() -> Self {
		Self {
			section_spacing: 0.12, panel_padding: 0.05,
			panel_position: Vec2::new(0.0, 0.0), panel_size: Vec2::new(0.0, 0.0),
			areas: Vec::new(),
		}
	}

	#[inline] pub const fn contains_point(&self, x: f32, y: f32) -> bool {
		let (px, py) = (self.panel_position.x, self.panel_position.y);
		let (w, h) = (self.panel_size.x, self.panel_size.y);
		x >= px && x <= px + w && y >= py && y <= py + h
	}
	
	fn calculate_panel_bounds(&mut self, areas: &[AreaLayout]) {
		if areas.is_empty() {
			self.panel_size = Vec2::new(0.0, 0.0);
			self.panel_position = Vec2::new(0.0, 0.0);
			return;
		}
		
		let mut min_x = f32::MAX;
		let mut min_y = f32::MAX;
		let mut max_x = f32::MIN;
		let mut max_y = f32::MIN;
		
		for area in areas {
			if area.rows > 0 && area.columns > 0 {
				let (left, bottom, right, top) = area.get_bounds();
				min_x = min_x.min(left);
				min_y = min_y.min(bottom);
				max_x = max_x.max(right);
				max_y = max_y.max(top);
			}
		}
		
		// Add padding around all areas
		min_x -= self.panel_padding;
		min_y -= self.panel_padding;
		max_x += self.panel_padding;
		max_y += self.panel_padding;
		
		self.panel_size = Vec2::new(max_x - min_x, max_y - min_y);
		self.panel_position = Vec2::new(min_x, min_y); // bottom-left corner
	}
	
	/// Debug method to check for overlaps between areas
	pub fn check_overlaps(&self, inv_state: InvState) -> Vec<String> {
		let mut overlaps = Vec::new();
		let areas = self.get_areas_for_inv_state(inv_state);
		
		for (i, area1) in areas.iter().enumerate() {
			for (j, area2) in areas.iter().enumerate() {
				if i >= j { continue; }
				
				let (l1, b1, r1, t1) = area1.get_bounds();
				let (l2, b2, r2, t2) = area2.get_bounds();
				
				// Check if rectangles overlap
				if r1 <= l2 || r2 <= l1 || t1 <= b2 || t2 <= b1 { continue; }
				
				overlaps.push(format!("Areas {} and {} overlap", i, j));
			}
		}
		
		overlaps
	}

	pub fn handle_click(&self, inv_state: InventoryUIState, x: f32, y: f32) -> ClickResult {
		if !self.contains_point(x, y) { return ClickResult::OutsidePanel; }
		
		let inv:InvState = match inv_state {
			InventoryUIState::Storage { inv, .. } |
			InventoryUIState::Crafting { inv, .. } |
			InventoryUIState::Player { inv } => inv,
		};

		let mut areas = self.get_areas_for_ui_state(inv_state);
		areas.extend(self.get_areas_for_inv_state(inv));

		for area in areas {
			if !area.contains_point(x, y) { continue; }

			for row in 0..area.rows {
				for col in 0..area.columns {
					if !area.slot_contains_point(row, col, x, y) { continue; }

					return ClickResult::SlotClicked { area_type: area.name, slot: (row, col) };
				}
			}
			return ClickResult::SlotMissed { area_type: area.name };
		}
		
		ClickResult::SlotMissed { area_type: AreaType::Panel }
	}
	
	// Helper for getting inventory-specific areas
	pub fn get_areas_for_inv_state(&self, inv_state: InvState) -> Vec<AreaLayout> {
		self.areas.iter().filter(|area| match (inv_state, area.name) {
			(InvState::All, AreaType::Inventory | AreaType::Armor | AreaType::Hotbar) => true,
			(InvState::Items, AreaType::Inventory | AreaType::Hotbar) => true,
			(InvState::Armor, AreaType::Armor) => true,
			(InvState::Inventory, AreaType::Inventory) => true,
			(InvState::Hotbar, AreaType::Hotbar) => true,
			_ => false,
		}).cloned().collect()  // Clone the areas
	}
	pub fn get_areas_for_ui_state(&self, inv_state: InventoryUIState) -> Vec<AreaLayout> {
		self.areas.iter().filter(|area| match (inv_state, area.name) {
			(InventoryUIState::Storage { inv: _, size: _ } , AreaType::Storage) => true,
			(InventoryUIState::Crafting { inv: _, size: _, result: _ } , AreaType::Input | AreaType::Output) => true,
			//(InventoryUIState::Player { inv: _ } , _) => true,
			_ => false,
		}).cloned().collect()  // Clone the areas
	}
}

#[derive(Clone, PartialEq)]
pub enum ClickResult {
	SlotClicked { area_type: AreaType, slot: (u8, u8) },
	SlotMissed { area_type: AreaType },
	OutsidePanel,
}

impl UIManager {
	pub fn setup_inventory_ui(&mut self) {
		let mut inventory = ptr::get_gamestate().player_mut().inventory_mut();

		if let UIState::Inventory(state) = self.state.clone() {
			let layout = match state {
				InventoryUIState::Player { inv } => InventoryLayout::calculate_for_player(inv, &mut inventory),
				InventoryUIState::Storage { inv, size } => InventoryLayout::calculate_for_storage(size, inv, &mut inventory),
				InventoryUIState::Crafting { inv, size, result } => InventoryLayout::calculate_for_crafting(size, result, inv, &mut inventory),
			};
			
			inventory.set_layout(&layout);
			self.add_main_panel(&layout);
			
			match state {
				InventoryUIState::Player { inv } => {
					self.add_player_buttons(&layout);
					self.create_inventory_slots(inv, &inventory);
				}
				InventoryUIState::Storage { inv, .. } => {
					// Create storage area with actual storage data
					self.create_storage_area(&layout);
					self.create_inventory_slots(inv, &inventory);
				}
				InventoryUIState::Crafting { inv, .. } => {
					// Create crafting areas with actual crafting data
					self.create_crafting_areas(&layout);
					self.create_inventory_slots(inv, &inventory);
				}
			}
		} else if UIState::InGame == self.state.clone() {
			let layout = InventoryLayout::calculate_for_player(InvState::Hotbar, &mut inventory);
			inventory.set_layout(&layout);
			self.create_inventory_slots(InvState::Hotbar, &inventory);
		}
	}

	// New method to handle storage UI using actual storage container
	fn create_storage_area(&mut self, layout: &InventoryLayout) {
		let Some(storage_area) = layout.areas.iter().find(|a| a.name == AreaType::Storage) else { return; };

		// Get actual storage data from game state (you'll need to implement this)
		let storage_items = self.get_storage_container(storage_area); // This should return &ItemContainer
		self.create_area_slots(&storage_area, &storage_items);
	}

	// New method to handle crafting UI using actual crafting containers
	fn create_crafting_areas(&mut self, layout: &InventoryLayout) {
		for area_type in [AreaType::Input, AreaType::Output] {
			let Some(area) = layout.areas.iter().find(|a| a.name == area_type) else { continue; };

			// Get actual crafting data from game state
			//let crafting_items = self.get_crafting_container(area_type); // This should return &ItemContainer
			let crafting_items = ItemContainer::new(area.rows, area.columns);
			self.create_area_slots(&area, &crafting_items);
		}
	}
	fn get_storage_container(&self, area: &AreaLayout) -> ItemContainer {
		// Return reference to actual storage container from game state
		// This is a placeholder - implement based on your storage system

		let mut storage_items = ItemContainer::new(area.rows, area.columns);
		let _ = storage_items.update_items(|idx, _|
			Some(ItemStack::new(ItemStack::lut_by_index(idx).name.into()))
		);
		storage_items
	}
	
	fn add_main_panel(&mut self, layout: &InventoryLayout) {
		let inv_config = &ptr::get_settings().inv_config;
		let panel = UIElement::panel(self.next_id())
			.with_position(layout.panel_position)
			.with_size(layout.panel_size)
			.with_style(&inv_config.panel_bg)
			.with_z_index(3);
		self.add_element(panel);
	}
	fn add_player_buttons(&mut self, layout: &InventoryLayout) {
		let theme = &ptr::get_settings().ui_theme;
		let w = 0.12; let h = SLOT/2.;
		let version = UIElement::label(self.next_id(), env!("CARGO_PKG_VERSION").into())
			.with_position(Vec2::new(layout.panel_position.x + layout.panel_size.x - w, layout.panel_position.y - h - PADDING))
			.with_size(Vec2::new(w, h))
			.with_style(&theme.labels.extra())
			.with_z_index(8);
		self.add_element(version);
	}

	// Updated method to better utilize inventory data
	fn create_inventory_slots(&mut self, inv_state: InvState, inventory: &Inventory) {
		let Some(layout) = inventory.get_layout().clone() else { return; };

		for area in layout.get_areas_for_inv_state(inv_state) {
			let items = inventory.get_area(&area.name);
			self.create_area_slots(&area, items);
			
			// Enhanced hotbar highlighting using actual inventory state
			if area.name != AreaType::Hotbar || UIState::InGame != self.state.clone() { continue; }
			
			self.hotbar_selection_highlight(inventory);
		}
		
		// Add cursor item display if player is holding something
		if let Some(cursor_item) = inventory.get_cursor() {
			let (mouse_x, mouse_y) = ptr::get_state().converted_mouse_position();
			self.cursor_item_display(mouse_x, mouse_y, cursor_item);
		}
	}
	// New method for better hotbar selection highlighting
	pub fn hotbar_selection_highlight(&mut self, inventory: &Inventory) {
		let selected_index = inventory.selected_index();
		let Some(layout) = inventory.get_layout() else { return; };

		let binding = layout.get_areas_for_inv_state(InvState::Hotbar);
		let Some(area) = binding.first() else { return; };

		let col = selected_index as u8 % area.columns;
		let row = selected_index as u8 / area.columns;
		if !(row < area.rows && col < area.columns) { return; };
		
		let (x, y) = area.get_slot_position(row, col);

		// Check if we have the correct focused element
		let focus_state = self.get_focused_state();
		if matches!(focus_state, FocusState::HotbarOverlay { .. }) {
			let Some(element) = self.get_focused_element_mut() else { return; };
			element.set_position(Vec2::new(x, y));
			return;
		}
		
		// If we get here, either no focused element or wrong type
		let id = self.next_id();
		let slot = UIElement::panel(id)
			.with_position(Vec2::new(x, y))
			.with_size(Vec2::new(SLOT, SLOT))
			.with_style(&ptr::get_settings().ui_theme.panels.nice.with_border_width(0.012))
			.with_z_index(4);
		self.add_element(slot);

		self.set_focused_state(FocusState::HotbarOverlay { id });
	}
	// New method to display item being held by cursor
	pub fn cursor_item_display(&mut self, x:f32, y:f32, cursor_item: &ItemStack) {
		// You'll need to get mouse position from your input system
		let (x,y) = (x - SLOT/2.0, y - SLOT/2.0);
		let origo = Vec2::new(x , y);
		// Check if we have the correct focused element
		if matches!(self.get_focused_state(), FocusState::Item { .. }) {
			let Some(element) = self.get_focused_element_mut() else { return; };
			element.set_position(origo);

			let childs = self.elements_with_parent_mut(self.get_focused_state().id());
			for child in childs {
				let offset:Vec2 = child.parent.pos();
				child.set_position(origo + offset);
			}
			return; // othervise it will create the element again 
		}

		// If we get here, either no focused element or wrong type

		let id = self.create_item_display(x, y, cursor_item, 10);

		self.set_focused_state(FocusState::Item { id });
	}

	fn create_area_slots(&mut self, area: &AreaLayout, items: &ItemContainer) {
		if area.rows == 0 || area.columns == 0 { return; }
		let config = &ptr::get_settings();
		
		for row in 0..area.rows {
			for col in 0..area.columns {
				let (x, y) = area.get_slot_position(row, col);

				let slot = UIElement::panel(self.next_id())
					.with_position(Vec2::new(x, y))
					.with_size(Vec2::new(SLOT, SLOT))
					.with_style(&config.inv_config.get_style(area.name))
					.with_z_index(5);
				self.add_element(slot);

				let Some(item) = items.get(row as usize * area.columns as usize + col as usize) else { continue; };

				self.create_item_display(x, y, item, 7);
			}
		}
	}

	fn create_item_display(&mut self, x:f32, y:f32, item: &ItemStack, z:i32) -> usize {
		let id = self.next_id();
		let item_display = UIElement::image(id, item.icon_path().into())
			.with_position(Vec2::new(x, y))
			.with_size(Vec2::new(SLOT, SLOT))
			.with_style(&ptr::get_settings().ui_theme.images.basic)
			.with_z_index(z);
		self.add_element(item_display);

		// Add quantity display for stackable items
		if item.stack() == 1 { return id; }

		let quantity_text = UIElement::label(self.next_id(), item.stack.to_string().into())
			.with_position(Vec2::new(x + SLOT * 0.3, y))
			.with_size(Vec2::new(SLOT * 0.7, SLOT * 0.6))
			.with_text_color(Solor::Black.i())
			.with_z_index(z+1)
			.with_parent_off(id.clone(), Vec2::new(SLOT * 0.3, 0.));
		self.add_element(quantity_text);

		id
	}
}
