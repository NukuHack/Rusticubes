

use crate::ui::{
	manager::{UIManager, UIState},
	element::UIElement,
};
use crate::game::inventory as inv;

const SLOT: f32 = 0.08;


#[derive(PartialEq, Clone, Copy)]
pub enum InventoryUIState {
	Player {
		inv: InvState,
		// Other player-specific fields
	},
	Storage {
		inv: InvState,
		size: SlotCount,
		// Other storage-specific fields like storage_id
	},
	Crafting {
		inv: InvState,
		size: SlotCount,
		result: SlotCount,
		// Crafting-specific fields
	},
}

impl Default for InventoryUIState {
	fn default() -> Self {
		Self::Player { inv: InvState::default() }
	}
}
impl InventoryUIState {
	pub fn def_pl() -> Self {
		Self::default()
	}
	pub fn new_pl(inv: InvState) -> Self {
		Self::Player { inv }
	}
	pub fn def_st() -> Self {
		Self::new_st(InvState::Items, SlotCount::MED)
	}
	pub fn new_st(inv: InvState, size: SlotCount) -> Self {
		Self::Storage { inv, size }
	}
	pub fn def_cr() -> Self {
		Self::new_cr(InvState::Items, SlotCount::custom(3,3), SlotCount::custom(1,1))
	}
	pub fn new_cr(inv: InvState, size: SlotCount, result: SlotCount) -> Self {
		Self::Crafting { inv, size, result }
	}
}

#[derive(PartialEq, Clone, Copy, Default)]
pub enum InvState {
	#[default]
	All,    // everything
	Items,  // hotbar and items
	Armor,  // only equipables
	Inner,  // only inner items - inventory but not in hotbar
	Hotbar, // hotbar
	None,   // ofc nothing
}

#[derive(PartialEq, Clone, Copy)]
pub struct SlotCount {
	pub rows: u8,
	pub columns: u8,
}

impl Default for SlotCount {
	fn default() -> Self {
		Self::SMALL
	}
}

impl SlotCount {
	// predefined sizes
	pub const NONE: Self = Self { rows: 0, columns: 0 }; // 0
	pub const TINY: Self = Self { rows: 3, columns: 5 }; // 15
	pub const SMALL: Self = Self { rows: 3, columns: 7 }; // 21
	pub const MED: Self = Self { rows: 5, columns: 9 }; // 45
	pub const BIG: Self = Self { rows: 6, columns: 12 }; // 72
	pub const GIANT: Self = Self { rows: 7, columns: 13 }; // 101

	pub fn total(&self) -> usize {
		self.rows as usize * self.columns as usize
	}

	pub fn custom(height: u8, width: u8) -> Self {
		Self {
			rows: height,
			columns: width,
		}
	}
}




#[derive(Debug, Clone)]
pub struct InventoryLayout {
	pub slot_size: f32,
	pub padding: f32,
	pub section_spacing: f32,
	pub panel_padding: f32,
	pub panel_position: (f32, f32), // bottom-left corner
	pub panel_size: (f32, f32),
	
	// Calculated positions for different sections
	pub storage_area: AreaLayout,
	pub input_area: AreaLayout,
	pub result_area: AreaLayout,
	pub inv_area: AreaLayout,
	pub hotbar_area: AreaLayout,
	pub armor_area: AreaLayout,
}

#[derive(Debug, Clone)]
pub struct AreaLayout {
	pub position: (f32, f32), // bottom-left corner of the area
	pub size: (f32, f32),
	pub rows: u8,
	pub columns: u8,
}

impl Default for InventoryLayout {
	fn default() -> Self {
		Self {
			slot_size: SLOT,
			padding: 0.01,
			section_spacing: 0.12,
			panel_padding: 0.05,
			panel_position: (0.0, 0.0),
			panel_size: (0.0, 0.0),
			storage_area: AreaLayout::default(),
			input_area: AreaLayout::default(),
			result_area: AreaLayout::default(),
			inv_area: AreaLayout::default(),
			hotbar_area: AreaLayout::default(),
			armor_area: AreaLayout::default(),
		}
	}
}

impl Default for AreaLayout {
	fn default() -> Self {
		Self {
			position: (0.0, 0.0),
			size: (0.0, 0.0),
			rows: 0,
			columns: 0,
		}
	}
}

impl AreaLayout {
	pub fn new(rows: u8, columns: u8, center_pos: (f32, f32), slot_size: f32, padding: f32) -> Self {
		let width = (columns as f32 * slot_size) + ((columns.saturating_sub(1)) as f32 * padding);
		let height = (rows as f32 * slot_size) + ((rows.saturating_sub(1)) as f32 * padding);
		
		// Convert center position to bottom-left corner
		let position = (
			center_pos.0 - width / 2.0,
			center_pos.1 - height / 2.0
		);
		
		Self {
			position,
			size: (width, height),
			rows,
			columns,
		}
	}
	
	/// Get the bounds of this area (left, bottom, right, top)
	pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
		let (x, y) = self.position;
		let (w, h) = self.size;
		(x, y, x + w, y + h)
	}
	
	/// Get the center position of this area
	pub fn get_center(&self) -> (f32, f32) {
		let (x, y) = self.position;
		let (w, h) = self.size;
		(x + w / 2.0, y + h / 2.0)
	}
	
	/// Get the position for a specific slot (row, col) - returns bottom-left corner
	pub fn get_slot_position(&self, row: u8, col: u8, slot_size: f32, padding: f32) -> (f32, f32) {
		let x = self.position.0 + (col as f32 * (slot_size + padding));
		let y = self.position.1 + ((self.rows - 1 - row) as f32 * (slot_size + padding));
		(x, y)
	}
}

impl InventoryLayout {
	pub fn calculate_for_player(inv_state: InvState) -> Self {
		let mut layout = Self::default();
		
		// Calculate areas with center positions first
		let inv_center = (0.0, 0.0);
		let hotbar_center = (0.0, inv_center.1 - layout.section_spacing);
		let armor_center = (inv_center.0 - layout.section_spacing * 2.0, inv_center.1);
		
		layout.inv_area = AreaLayout::new(
			inv::inv_row(), inv::inv_col(), 
			inv_center, layout.slot_size, layout.padding
		);
		
		layout.hotbar_area = AreaLayout::new(
			1, inv::hotbar(), 
			hotbar_center, layout.slot_size, layout.padding
		);
		
		layout.armor_area = AreaLayout::new(
			inv::armor(), 1, 
			armor_center, layout.slot_size, layout.padding
		);

		let areas = layout.get_active_areas(inv_state);
		layout.calculate_panel_bounds(&areas);
		
		layout
	}
	
	pub fn calculate_for_storage(storage_size: SlotCount, inv_state: InvState) -> Self {
		let mut layout = Self::default();
		
		// Calculate areas with center positions
		let storage_center = (0.0, layout.section_spacing);
		let inv_center = (0.0, -layout.section_spacing);
		let hotbar_center = (0.0, inv_center.1 - layout.section_spacing);
		let armor_center = (inv_center.0 - layout.section_spacing * 2.0, inv_center.1);
		
		layout.storage_area = AreaLayout::new(
			storage_size.rows, storage_size.columns,
			storage_center, layout.slot_size, layout.padding
		);
		
		layout.inv_area = AreaLayout::new(
			inv::inv_row(), inv::inv_col(), 
			inv_center, layout.slot_size, layout.padding
		);
		
		layout.hotbar_area = AreaLayout::new(
			1, inv::hotbar(), 
			hotbar_center, layout.slot_size, layout.padding
		);
		
		layout.armor_area = AreaLayout::new(
			inv::armor(), 1, 
			armor_center, layout.slot_size, layout.padding
		);
		
		let mut areas = vec![layout.storage_area.clone()];
		areas.extend(layout.get_active_areas(inv_state));
		
		layout.calculate_panel_bounds(&areas);
		layout
	}
	
	pub fn calculate_for_crafting(crafting_size: SlotCount, result_size: SlotCount, inv_state: InvState) -> Self {
		let mut layout = Self::default();
		
		// Calculate crafting areas side by side at the top
		let crafting_spacing = layout.section_spacing * 1.5;
		let crafting_y = layout.section_spacing;
		
		let input_center = (-crafting_spacing, crafting_y);
		let result_center = (crafting_spacing, crafting_y);
		let inv_center = (0.0, -layout.section_spacing);
		let hotbar_center = (0.0, inv_center.1 - layout.section_spacing);
		let armor_center = (inv_center.0 - layout.section_spacing * 2.0, inv_center.1);
		
		layout.input_area = AreaLayout::new(
			crafting_size.rows, crafting_size.columns,
			input_center, layout.slot_size, layout.padding
		);
		
		layout.result_area = AreaLayout::new(
			result_size.rows, result_size.columns,
			result_center, layout.slot_size, layout.padding
		);
		
		layout.inv_area = AreaLayout::new(
			inv::inv_row(), inv::inv_col(), 
			inv_center, layout.slot_size, layout.padding
		);
		
		layout.hotbar_area = AreaLayout::new(
			1, inv::hotbar(), 
			hotbar_center, layout.slot_size, layout.padding
		);
		
		layout.armor_area = AreaLayout::new(
			inv::armor(), 1, 
			armor_center, layout.slot_size, layout.padding
		);
		
		let mut areas = vec![layout.input_area.clone(), layout.result_area.clone()];
		areas.extend(layout.get_active_areas(inv_state));
		
		layout.calculate_panel_bounds(&areas);
		layout
	}
	
	fn get_active_areas(&self, inv_state: InvState) -> Vec<AreaLayout> {
		match inv_state {
			InvState::Armor => vec![self.armor_area.clone()],
			InvState::Inner => vec![self.inv_area.clone()],
			InvState::Hotbar => vec![self.hotbar_area.clone()],
			InvState::Items => vec![self.inv_area.clone(), self.hotbar_area.clone()],
			InvState::All => vec![
				self.inv_area.clone(), 
				self.armor_area.clone(), 
				self.hotbar_area.clone()
			],
			InvState::None => vec![],
		}
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
	
	/// Get the panel's center position
	pub fn get_panel_center(&self) -> (f32, f32) {
		(
			self.panel_position.0 + self.panel_size.0 / 2.0,
			self.panel_position.1 + self.panel_size.1 / 2.0
		)
	}
}


impl UIManager {
	pub fn setup_inventory_ui(&mut self) {
		self.clear_elements();

		if let UIState::Inventory(state) = self.state.clone() {
			let layout = match state {
				InventoryUIState::Player { inv } => {
					InventoryLayout::calculate_for_player(inv)
				}
				InventoryUIState::Storage { inv, size } => {
					InventoryLayout::calculate_for_storage(size, inv)
				}
				InventoryUIState::Crafting { inv, size, result } => {
					InventoryLayout::calculate_for_crafting(size, result, inv)
				}
			};
						
			// Add main panel
			self.add_main_panel(&layout);
			
			// Add specific inventory elements
			match state {
				InventoryUIState::Player { inv } => {
					self.add_player_buttons(&layout);
					self.create_inventory_slots(inv, &layout);
				}
				InventoryUIState::Storage { inv, size: _ } => {
					self.create_area_slots(&layout.storage_area, (40, 40, 60), (80, 80, 120, 255));
					self.create_inventory_slots(inv, &layout);
				}
				InventoryUIState::Crafting { inv, size: _, result: _ } => {
					self.create_area_slots(&layout.input_area, (40, 40, 60), (80, 80, 120, 255));
					self.create_area_slots(&layout.result_area, (60, 80, 60), (80, 120, 80, 255));
					self.create_inventory_slots(inv, &layout);
				}
			}
		}
	}
	
	fn add_main_panel(&mut self, layout: &InventoryLayout) {
		let main_panel = UIElement::panel(self.next_id())
			.with_position(layout.panel_position.0, layout.panel_position.1)
			.with_size(layout.panel_size.0, layout.panel_size.1)
			.with_color(25, 25, 40)
			.with_border((60, 60, 90, 255), 0.008)
			.with_z_index(1);
		self.add_element(main_panel);
	}

	fn add_player_buttons(&mut self, layout: &InventoryLayout) {        
		// Version label at bottom-right
		let version_width = 0.12;
		let version_height = SLOT/2.;
		let version_x = layout.panel_position.0 + layout.panel_size.0 - version_width;
		let version_y = layout.panel_position.1 - version_height - 0.02;
		
		let version = UIElement::label(self.next_id(), format!("v{}", env!("CARGO_PKG_VERSION")))
			.with_position(version_x, version_y)
			.with_size(version_width, version_height)
			.with_color(30, 30, 45)
			.with_text_color(200, 230, 255)
			.with_border((60, 80, 120, 170), 0.003)
			.with_z_index(8);
		self.add_element(version);
	}

	fn create_inventory_slots(&mut self, inv_state: InvState, layout: &InventoryLayout) {
		match inv_state {
			InvState::All => {
				self.create_area_slots(&layout.armor_area, (60, 60, 80), (100, 100, 140, 255));
				self.create_area_slots(&layout.inv_area, (40, 40, 60), (80, 80, 120, 255));
				self.create_area_slots(&layout.hotbar_area, (50, 50, 70), (90, 90, 130, 255));
			}
			InvState::Items => {
				self.create_area_slots(&layout.inv_area, (40, 40, 60), (80, 80, 120, 255));
				self.create_area_slots(&layout.hotbar_area, (50, 50, 70), (90, 90, 130, 255));
			}
			InvState::Armor => {
				self.create_area_slots(&layout.armor_area, (60, 60, 80), (100, 100, 140, 255));
			}
			InvState::Inner => {
				self.create_area_slots(&layout.inv_area, (40, 40, 60), (80, 80, 120, 255));
			}
			InvState::Hotbar => {
				self.create_area_slots(&layout.hotbar_area, (50, 50, 70), (90, 90, 130, 255));
			}
			InvState::None => {}
		}
	}

	fn create_area_slots(&mut self, area: &AreaLayout, slot_color: (u8, u8, u8), border_color: (u8, u8, u8, u8)) {
		if area.rows == 0 || area.columns == 0 {
			return;
		}
		
		for row in 0..area.rows {
			for col in 0..area.columns {
				let (x, y) = area.get_slot_position(row, col, SLOT, 0.02);
				
				let slot = UIElement::panel(self.next_id())
					.with_position(x, y)
					.with_size(SLOT, SLOT)
					.with_color(slot_color.0, slot_color.1, slot_color.2)
					.with_border(border_color, 0.003)
					.with_z_index(2);
				
				self.add_element(slot);
			}
		}
	}
}