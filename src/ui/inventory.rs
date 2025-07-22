
use crate::ext::ptr;
use crate::ui::{manager::{UIManager, UIState}, element::UIElement};
use crate::game::inventory as inv;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum InventoryUIState {
	Player { inv: InvState },
	Storage { inv: InvState, size: SlotCount },
	Crafting { inv: InvState, size: SlotCount, result: SlotCount },
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

builder!(StorageInvBuilder { inv: InvState = InvState::Items, size: SlotCount = SlotCount::MED });
impl StorageInvBuilder {
	#[inline] pub const fn inv(mut self, inv: InvState) -> Self { self.inv = inv; self }
	#[inline] pub const fn size(mut self, size: SlotCount) -> Self { self.size = size; self }
	#[inline] pub const fn b(self) -> InventoryUIState {
		InventoryUIState::Storage { inv: self.inv, size: self.size }
	}
}

builder!(CraftingInvBuilder { inv: InvState = InvState::Items, size: SlotCount = SlotCount::custom(3, 3), result: SlotCount = SlotCount::custom(1, 1) });
impl CraftingInvBuilder {
	#[inline] pub const fn inv(mut self, inv: InvState) -> Self { self.inv = inv; self }
	#[inline] pub const fn input(mut self, size: SlotCount) -> Self { self.size = size; self }
	#[inline] pub const fn result(mut self, result: SlotCount) -> Self { self.result = result; self }
	#[inline] pub const fn b(self) -> InventoryUIState {
		InventoryUIState::Crafting { inv: self.inv, size: self.size, result: self.result }
	}
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum InvState { All, Items, Armor, Inventory, Hotbar, None }

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AreaType { Panel, Inventory, Hotbar, Armor, Storage, Input, Output }

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct SlotCount { pub rows: u8, pub columns: u8 }

impl SlotCount {
	pub const NONE: Self = Self { rows: 0, columns: 0 };
	pub const TINY: Self = Self { rows: 3, columns: 5 };
	pub const SMALL: Self = Self { rows: 3, columns: 7 };
	pub const MED: Self = Self { rows: 5, columns: 9 };
	pub const BIG: Self = Self { rows: 6, columns: 12 };
	pub const GIANT: Self = Self { rows: 7, columns: 13 };

	#[inline] pub const fn default() -> Self { Self::SMALL }
	#[inline] pub const fn total(&self) -> usize { self.rows as usize * self.columns as usize }
	#[inline] pub const fn custom(rows: u8, columns: u8) -> Self { Self { rows, columns } }
}

const SLOT: f32 = 0.08;
const PADDING: f32 = 0.02;

#[derive(Clone, PartialEq, Debug)]
pub struct InventoryLayout {
	pub section_spacing: f32,
	pub panel_padding: f32,
	pub panel_position: (f32, f32), // bottom-left corner
	pub panel_size: (f32, f32),
	
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
	pub fn calculate_for_player(inv_state: InvState, inv_lay: &mut inv::Inventory) -> Self {
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
		let mut layout = Self::default();
		let inv_lay = inv_lay.clone();
		
		// Special case for hotbar only
		if inv_state == InvState::Hotbar {
			layout.areas.push(AreaLayout::new(1, inv_lay.hotbar(), (0.0, -0.8), AreaType::Hotbar));
			layout.finalize_layout(inv_state, &[]);
			return layout;
		}
		
		let (inv_area, hotbar_area, armor_area) = Self::create_player_areas(&inv_lay);
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
	pub fn calculate_for_storage(storage_size: SlotCount, inv_state: InvState, inv_lay: &mut inv::Inventory) -> Self {
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
		
		let storage_area = (storage_size.rows, storage_size.columns);
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
	pub fn calculate_for_crafting(crafting_size: SlotCount, result_size: SlotCount, inv_state: InvState, inv_lay: &mut inv::Inventory) -> Self {
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
		
		let input_area = (crafting_size.rows, crafting_size.columns);
		let output_area = (result_size.rows, result_size.columns);
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
	fn create_player_areas(inv_lay: &inv::Inventory) -> ((u8, u8), (u8, u8), (u8, u8)) {
		(
			(inv_lay.inv_row(), inv_lay.inv_col()),
			(1, inv_lay.hotbar()),
			(inv_lay.armor(), 1),
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
			panel_position: (0.0, 0.0), panel_size: (0.0, 0.0),
			areas: vec![],
		}
	}

	#[inline] pub const fn contains_point(&self, x: f32, y: f32) -> bool {
		let (px, py) = self.panel_position;
		let (w, h) = self.panel_size;
		x >= px && x <= px + w && y >= py && y <= py + h
	}
	
	fn calculate_panel_bounds(&mut self, areas: &[AreaLayout]) {
		if areas.is_empty() {
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
		
		self.panel_size = (max_x - min_x, max_y - min_y);
		self.panel_position = (min_x, min_y); // bottom-left corner
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
				if !(r1 <= l2 || r2 <= l1 || t1 <= b2 || t2 <= b1) {
					overlaps.push(format!("Areas {} and {} overlap", i, j));
				}
			}
		}
		
		overlaps
	}

	pub fn handle_click(&self, inv_state: InventoryUIState, x: f32, y: f32) -> ClickResult {
		if !self.contains_point(x, y) { return ClickResult::OutsidePanel; }
		
		let (extra,inv) = match inv_state {
			InventoryUIState::Storage { inv,.. } => (vec![AreaType::Storage], inv),
			InventoryUIState::Crafting {inv, .. } => (vec![AreaType::Input, AreaType::Output], inv),
			InventoryUIState::Player { inv } => (vec![], inv),
		};

		fn get_active_areas<'a>(areas: &'a [AreaLayout], extra: &'a [AreaType]) -> Vec<&'a AreaLayout> {
			areas.iter().filter(|a| extra.contains(&a.name)).collect()
		}
		
		let mut areas = get_active_areas(&self.areas, &extra);
		let binding = self.get_areas_for_inv_state(inv);
		areas.extend(&binding);

		for area in areas {
			if area.contains_point(x, y) {
				for row in 0..area.rows {
					for col in 0..area.columns {
						if area.slot_contains_point(row, col, x, y) {
							return ClickResult::SlotClicked { area_type: area.name, slot: (row, col) };
						}
					}
				}
				return ClickResult::SlotMissed { area_type: area.name };
			}
		}
		
		ClickResult::SlotMissed { area_type: AreaType::Panel }
	}
	
	// Helper for getting inventory-specific areas
	fn get_areas_for_inv_state(&self, inv_state: InvState) -> Vec<AreaLayout> {
		self.areas.iter().filter(|area| match (inv_state, area.name) {
			(InvState::All, AreaType::Inventory | AreaType::Armor | AreaType::Hotbar) => true,
			(InvState::Items, AreaType::Inventory | AreaType::Hotbar) => true,
			(InvState::Armor, AreaType::Armor) => true,
			(InvState::Inventory, AreaType::Inventory) => true,
			(InvState::Hotbar, AreaType::Hotbar) => true,
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
		let inv_lay = ptr::get_gamestate().player_mut().inventory_mut();

		if let UIState::Inventory(state) = self.state.clone() {
			let layout = match state {
				InventoryUIState::Player { inv } => InventoryLayout::calculate_for_player(inv, inv_lay),
				InventoryUIState::Storage { inv, size } => InventoryLayout::calculate_for_storage(size, inv, inv_lay),
				InventoryUIState::Crafting { inv, size, result } => InventoryLayout::calculate_for_crafting(size, result, inv, inv_lay),
			};
			
			inv_lay.set_layout(&layout);
			self.add_main_panel(&layout);
			
			match state {
				InventoryUIState::Player { inv } => {
					self.add_player_buttons(&layout);
					self.create_inventory_slots(inv, &layout);
				}
				InventoryUIState::Storage { inv, .. } => {
					self.create_area_slots(&layout.areas.iter().find(|a| a.name == AreaType::Storage).unwrap());
					self.create_inventory_slots(inv, &layout);
				}
				InventoryUIState::Crafting { inv, .. } => {
					for area_type in [AreaType::Input, AreaType::Output] {
						self.create_area_slots(&layout.areas.iter().find(|a| a.name == area_type).unwrap());
					}
					self.create_inventory_slots(inv, &layout);
				}
			}
		} else if UIState::InGame == self.state.clone() {
			let layout = InventoryLayout::calculate_for_player(InvState::Hotbar, inv_lay);
			self.create_inventory_slots(InvState::Hotbar, &layout);
		}
	}
	
	fn add_main_panel(&mut self, layout: &InventoryLayout) {
		let panel = UIElement::panel(self.next_id())
			.with_position(layout.panel_position.0, layout.panel_position.1)
			.with_size(layout.panel_size.0, layout.panel_size.1)
			//.with_style(&self.theme.inv.panel_bg)
			.with_z_index(3);
		self.add_element(panel);
	}

	fn add_player_buttons(&mut self, layout: &InventoryLayout) {        
		let w = 0.12; let h = SLOT/2.;
		let version = UIElement::label(self.next_id(), format!("v{}", env!("CARGO_PKG_VERSION")))
			.with_position(layout.panel_position.0 + layout.panel_size.0 - w, layout.panel_position.1 - h - PADDING)
			.with_size(w, h)
			.with_style(&self.theme.labels.extra())
			.with_z_index(8);
		self.add_element(version);
	}

	fn create_inventory_slots(&mut self, inv_state: InvState, layout: &InventoryLayout) {
		for area in layout.get_areas_for_inv_state(inv_state) {
			self.create_area_slots(&area);
		}
	}

	fn create_area_slots(&mut self, area: &AreaLayout) {
		if area.rows == 0 || area.columns == 0 { return; }
		
		for row in 0..area.rows {
			for col in 0..area.columns {
				let (x, y) = area.get_slot_position(row, col);
				let slot = UIElement::panel(self.next_id())
					.with_position(x, y)
					.with_size(SLOT, SLOT)
					//.with_style(self.theme.inv.get_style(area.name))
					.with_z_index(5);
				self.add_element(slot);
			}
		}
	}
}