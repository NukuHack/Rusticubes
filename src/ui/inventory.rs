

use crate::ext::ptr;
use crate::ui::{
	manager::{UIManager, UIState},
	element::UIElement,
};
use crate::game::inventory as inv;


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
// 3. Better builder pattern for InventoryUIState
impl InventoryUIState {
	#[inline]
	pub fn pl() -> PlayerInvBuilder {
		PlayerInvBuilder::new()
	}
	#[inline]
	pub fn str() -> StorageInvBuilder {
		StorageInvBuilder::new()
	}
	#[inline]
	pub fn craft() -> CraftingInvBuilder {
		CraftingInvBuilder::new()
	}
}
pub struct PlayerInvBuilder {
	inv: InvState,
}

impl PlayerInvBuilder {
	#[inline]
	fn new() -> Self {
		Self { inv: InvState::default() }
	}
	#[inline]
	pub fn inv(mut self, inv: InvState) -> Self {
		self.inv = inv;
		self
	}
	#[inline]
	pub fn b(self) -> InventoryUIState {
		InventoryUIState::Player { inv: self.inv }
	}
}
pub struct StorageInvBuilder {
	inv: InvState,
	size: SlotCount,
}
impl StorageInvBuilder {
	#[inline]
	fn new() -> Self {
		Self { 
			inv: InvState::Items, 
			size: SlotCount::MED 
		}
	}
	#[inline]
	pub fn inv(mut self, inv: InvState) -> Self {
		self.inv = inv;
		self
	}
	#[inline]
	pub fn size(mut self, size: SlotCount) -> Self {
		self.size = size;
		self
	}
	#[inline]
	pub fn b(self) -> InventoryUIState {
		InventoryUIState::Storage { 
			inv: self.inv, 
			size: self.size 
		}
	}
}
pub struct CraftingInvBuilder {
	inv: InvState,
	size: SlotCount,
	result: SlotCount,
}
impl CraftingInvBuilder {
	#[inline]
	fn new() -> Self {
		Self { 
			inv: InvState::Items, 
			size: SlotCount::custom(3, 3),
			result: SlotCount::custom(1, 1)
		}
	}
	#[inline]
	pub fn inv(mut self, inv: InvState) -> Self {
		self.inv = inv;
		self
	}
	#[inline]
	pub fn input(mut self, size: SlotCount) -> Self {
		self.size = size;
		self
	}
	#[inline]
	pub fn result(mut self, result: SlotCount) -> Self {
		self.result = result;
		self
	}
	#[inline]
	pub fn b(self) -> InventoryUIState {
		InventoryUIState::Crafting { 
			inv: self.inv, 
			size: self.size, 
			result: self.result 
		}
	}
}
impl Default for InventoryUIState {
	#[inline]
	fn default() -> Self {
		Self::Player { inv: InvState::default() }
	}
}


#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub enum InvState {
	#[default]
	All,    // everything
	Items,  // hotbar and items
	Armor,  // only equipables
	Inner,  // only inner items - inventory but not in hotbar
	Hotbar, // hotbar
	None,   // ofc nothing
}
#[derive(PartialEq, Clone, Copy, Default)]
pub enum AreaType {
	#[default]
	Panel,
	Inventory,
	Hotbar,
	Armor,

	Storage,
	Input,
	Output,
}
impl std::fmt::Debug for AreaType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Inventory => write!(f, "Inventory"),
			Self::Hotbar => write!(f, "Hotbar"),
			Self::Armor => write!(f, "Armor"),
			Self::Storage => write!(f, "Storage"),
			Self::Input => write!(f, "Input"),
			Self::Output => write!(f, "Output"),
			Self::Panel => write!(f, "Outside Panel"),
		}
	}
}

#[derive(PartialEq, Clone, Copy)]
pub struct SlotCount {
	pub rows: u8,
	pub columns: u8,
}

impl Default for SlotCount {
	#[inline]
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

	#[inline]
	pub fn total(&self) -> usize {
		self.rows as usize * self.columns as usize
	}

	#[inline]
	pub fn custom(height: u8, width: u8) -> Self {
		Self {
			rows: height,
			columns: width,
		}
	}
}

const SLOT: f32 = 0.08;
const PADDING: f32 = 0.02;

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryLayout {
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

#[derive(Debug, Clone, PartialEq)]
pub struct AreaLayout {
	pub name: AreaType,
	pub position: (f32, f32), // bottom-left corner of the area
	pub size: (f32, f32),
	pub rows: u8,
	pub columns: u8,
}

impl Default for InventoryLayout {
	#[inline]
	fn default() -> Self {
		Self {
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
	#[inline]
	fn default() -> Self {
		Self {
			name : AreaType::default(),
			position: (0.0, 0.0),
			size: (0.0, 0.0),
			rows: 0,
			columns: 0,
		}
	}
}

impl AreaLayout {
	#[inline]
	pub fn new(rows: u8, columns: u8, center_pos: (f32, f32), name: AreaType) -> Self {
		let width = (columns as f32 * SLOT) + ((columns.saturating_sub(1)) as f32 * PADDING);
		let height = (rows as f32 * SLOT) + ((rows.saturating_sub(1)) as f32 * PADDING);
		
		// Convert center position to bottom-left corner
		let position = (
			center_pos.0 - width / 2.0,
			center_pos.1 - height / 2.0
		);
		
		Self {
			name,
			position,
			size: (width, height),
			rows,
			columns,
		}
	}
	
	/// Get the bounds of this area (left, bottom, right, top)
	#[inline]
	pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
		let (x, y) = self.position;
		let (w, h) = self.size;
		(x, y, x + w, y + h)
	}
	#[inline]
	pub fn contains_point(&self, x: f32, y: f32) -> bool {
		let (min_x, min_y, max_x, max_y) = self.get_bounds();
		x >= min_x && x <= max_x && y >= min_y && y <= max_y
	}
	
	/// Get the center position of this area
	#[inline]
	pub fn get_center(&self) -> (f32, f32) {
		let (x, y) = self.position;
		let (w, h) = self.size;
		(x + w / 2.0, y + h / 2.0)
	}
	
	/// Get the position for a specific slot (row, col) - returns bottom-left corner
	#[inline]
	pub fn get_slot_position(&self, row: u8, col: u8) -> (f32, f32) {
		let x = self.position.0 + (col as f32 * (SLOT + PADDING));
		let y = self.position.1 + ((self.rows - 1 - row) as f32 * (SLOT + PADDING));
		(x, y)
	}
	/// Get the bounds of a single slot (left, bottom, right, top)
	#[inline]
	pub fn get_slot_bounds(&self, row: u8, col: u8) -> (f32, f32, f32, f32) {
		let (x, y) = self.get_slot_position(row, col);
		(x, y, x + SLOT, y + SLOT)
	}
	#[inline]
	pub fn slot_contains_point(&self, row: u8, col: u8, x: f32, y: f32) -> bool {
		let (min_x, min_y, max_x, max_y) = self.get_slot_bounds(row, col);
		x >= min_x && x <= max_x && y >= min_y && y <= max_y
	}
}

impl InventoryLayout {
	#[inline]
	pub fn calculate_for_player(inv_state: InvState, inv_lay: &mut inv::Inventory) -> Self {
		let mut layout = Self::default();
		let inv_lay = inv_lay.clone();
		
		// Create temporary areas to calculate proper spacing
		let temp_inv = AreaLayout::new(
			inv_lay.inv_row(), inv_lay.inv_col(), 
			(0.0, 0.0), AreaType::Inventory,
		);
		let temp_hotbar = AreaLayout::new(
			1, inv_lay.hotbar(), 
			(0.0, 0.0), AreaType::Hotbar,
		);
		let temp_armor = AreaLayout::new(
			inv_lay.armor(), 1, 
			(0.0, 0.0), AreaType::Armor,
		);
		
		// Calculate positions with proper spacing
		let inv_center = (0.0, 0.0);
		let hotbar_center = (0.0, inv_center.1 - temp_inv.size.1/2.0 - layout.section_spacing - temp_hotbar.size.1/2.0);
		let armor_center = (inv_center.0 - temp_inv.size.0/2.0 - layout.section_spacing - temp_armor.size.0/2.0, inv_center.1);
		
		layout.inv_area = AreaLayout::new(
			inv_lay.inv_row(), inv_lay.inv_col(), 
			inv_center, AreaType::Inventory,
		);
		
		layout.hotbar_area = AreaLayout::new(
			1, inv_lay.hotbar(), 
			hotbar_center, AreaType::Hotbar,
		);
		
		layout.armor_area = AreaLayout::new(
			inv_lay.armor(), 1, 
			armor_center, AreaType::Armor,
		);

		let areas = layout.get_active_areas(inv_state);
		layout.calculate_panel_bounds(&areas);
		
		layout
	}
	
	#[inline]
	pub fn calculate_for_storage(storage_size: SlotCount, inv_state: InvState, inv_lay: &mut inv::Inventory) -> Self {
		let mut layout = Self::default();
		let inv_lay = inv_lay.clone();
		
		// Create temporary areas to calculate proper spacing
		let temp_storage = AreaLayout::new(
			storage_size.rows, storage_size.columns,
			(0.0, 0.0), AreaType::Storage,
		);
		let temp_inv = AreaLayout::new(
			inv_lay.inv_row(), inv_lay.inv_col(), 
			(0.0, 0.0), AreaType::Inventory,
		);
		let temp_hotbar = AreaLayout::new(
			1, inv_lay.hotbar(), 
			(0.0, 0.0), AreaType::Hotbar,
		);
		let temp_armor = AreaLayout::new(
			inv_lay.armor(), 1, 
			(0.0, 0.0), AreaType::Armor,
		);
		
		// Calculate positions with proper spacing
		let storage_center = (0.0, temp_storage.size.1/2.0 + layout.section_spacing);
		let inv_center = (0.0, -temp_inv.size.1/2.0 - layout.section_spacing);
		let hotbar_center = (0.0, inv_center.1 - temp_inv.size.1/2.0 - layout.section_spacing - temp_hotbar.size.1/2.0);
		let armor_center = (inv_center.0 - temp_inv.size.0/2.0 - layout.section_spacing - temp_armor.size.0/2.0, inv_center.1);
		
		layout.storage_area = AreaLayout::new(
			storage_size.rows, storage_size.columns,
			storage_center, AreaType::Storage,
		);
		
		layout.inv_area = AreaLayout::new(
			inv_lay.inv_row(), inv_lay.inv_col(), 
			inv_center, AreaType::Inventory,
		);
		
		layout.hotbar_area = AreaLayout::new(
			1, inv_lay.hotbar(), 
			hotbar_center, AreaType::Hotbar,
		);
		
		layout.armor_area = AreaLayout::new(
			inv_lay.armor(), 1, 
			armor_center, AreaType::Armor,
		);
		
		let mut areas = vec![layout.storage_area.clone()];
		areas.extend(layout.get_active_areas(inv_state));
		
		layout.calculate_panel_bounds(&areas);
		layout
	}
	
	#[inline]
	pub fn calculate_for_crafting(crafting_size: SlotCount, result_size: SlotCount, inv_state: InvState, inv_lay: &mut inv::Inventory) -> Self {
		let mut layout = Self::default();
		let inv_lay = inv_lay.clone();
		
		// Create temporary areas to calculate proper spacing
		let temp_input = AreaLayout::new(
			crafting_size.rows, crafting_size.columns,
			(0.0, 0.0), AreaType::Input,
		);
		let temp_result = AreaLayout::new(
			result_size.rows, result_size.columns,
			(0.0, 0.0), AreaType::Output,
		);
		let temp_inv = AreaLayout::new(
			inv_lay.inv_row(), inv_lay.inv_col(), 
			(0.0, 0.0), AreaType::Inventory,
		);
		let temp_hotbar = AreaLayout::new(
			1, inv_lay.hotbar(), 
			(0.0, 0.0), AreaType::Hotbar,
		);
		let temp_armor = AreaLayout::new(
			inv_lay.armor(), 1, 
			(0.0, 0.0), AreaType::Armor,
		);
		
		// Calculate crafting areas side by side at the top with proper spacing
		let crafting_y = layout.section_spacing + temp_input.size.1/2.0f32.max(temp_result.size.1/2.0);
		let horizontal_spacing = (temp_input.size.0 + temp_result.size.0)/2.0 + layout.section_spacing;
		
		let input_center = (-horizontal_spacing/2.0, crafting_y);
		let result_center = (horizontal_spacing/2.0, crafting_y);
		let inv_center = (0.0, -temp_inv.size.1/2.0 - layout.section_spacing);
		let hotbar_center = (0.0, inv_center.1 - temp_inv.size.1/2.0 - layout.section_spacing - temp_hotbar.size.1/2.0);
		let armor_center = (inv_center.0 - temp_inv.size.0/2.0 - layout.section_spacing - temp_armor.size.0/2.0, inv_center.1);
		
		layout.input_area = AreaLayout::new(
			crafting_size.rows, crafting_size.columns,
			input_center, AreaType::Input,
		);
		
		layout.result_area = AreaLayout::new(
			result_size.rows, result_size.columns,
			result_center, AreaType::Output,
		);
		
		layout.inv_area = AreaLayout::new(
			inv_lay.inv_row(), inv_lay.inv_col(), 
			inv_center, AreaType::Inventory,
		);
		
		layout.hotbar_area = AreaLayout::new(
			1, inv_lay.hotbar(), 
			hotbar_center, AreaType::Hotbar,
		);
		
		layout.armor_area = AreaLayout::new(
			inv_lay.armor(), 1, 
			armor_center, AreaType::Armor,
		);
		
		let mut areas = vec![layout.input_area.clone(), layout.result_area.clone()];
		areas.extend(layout.get_active_areas(inv_state));
		
		layout.calculate_panel_bounds(&areas);
		layout
	}
	
	#[inline]
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
	#[inline]
	pub fn get_panel_center(&self) -> (f32, f32) {
		(
			self.panel_position.0 + self.panel_size.0 / 2.0,
			self.panel_position.1 + self.panel_size.1 / 2.0
		)
	}

	/// Debug method to check for overlaps between areas
	pub fn check_overlaps(&self, inv_state: InvState) -> Vec<String> {
		let mut overlaps = Vec::new();
		let areas = self.get_active_areas(inv_state);
		
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
	/// Get the bounds of the panel area (left, bottom, right, top)
	#[inline]
	pub fn get_panel_bounds(&self) -> (f32, f32, f32, f32) {
		let (x, y) = self.panel_position;
		let (w, h) = self.panel_size;
		(x, y, x + w, y + h)
	}
	#[inline]
	pub fn contains_point(&self, x: f32, y: f32) -> bool {
		let (min_x, min_y, max_x, max_y) = self.get_panel_bounds();
		x >= min_x && x <= max_x && y >= min_y && y <= max_y
	}
}

// 2. Better click handling with proper return types
#[derive(Debug, Clone, PartialEq)]
pub enum ClickResult {
	SlotClicked {
		area_type: AreaType,
		slot: (u8, u8),
	},
	SlotMissed {
		area_type: AreaType,
	},
	OutsidePanel,
}

impl InventoryLayout {
	pub fn handle_click(&self, inv_state: InventoryUIState, x: f32, y: f32) -> ClickResult {
		// Check if click is within panel bounds first
		if !self.contains_point(x, y) {
			return ClickResult::OutsidePanel;
		}

		let areas = self.get_areas_for_state(inv_state);
		
		for area in areas {
			if area.contains_point(x, y) {
				// Check each slot in the area
				for row in 0..area.rows {
					for col in 0..area.columns {
						if area.slot_contains_point(row, col, x, y) {
							return ClickResult::SlotClicked {
								area_type: area.name,
								slot: (row, col),
							};
						}
					}
				}
				// Clicked in area but not on any slot
				return ClickResult::SlotMissed {
					area_type: area.name,
				};
			}
		}
		
		return ClickResult::SlotMissed {
			area_type: AreaType::default(),
		};
	}

	// Centralized method to get areas for different states
	fn get_areas_for_state(&self, inv_state: InventoryUIState) -> Vec<&AreaLayout> {
		match inv_state {
			InventoryUIState::Player { inv } => {
				self.get_active_areas_ref(inv)
			},
			InventoryUIState::Storage { inv, .. } => {
				let mut areas = self.get_active_areas_ref(inv);
				areas.push(&self.storage_area);
				areas
			},
			InventoryUIState::Crafting { inv, .. } => {
				let mut areas = self.get_active_areas_ref(inv);
				areas.push(&self.input_area);
				areas.push(&self.result_area);
				areas
			},
		}
	}

	// Return references instead of clones for better performance
	fn get_active_areas_ref(&self, inv_state: InvState) -> Vec<&AreaLayout> {
		match inv_state {
			InvState::Armor => vec![&self.armor_area],
			InvState::Inner => vec![&self.inv_area],
			InvState::Hotbar => vec![&self.hotbar_area],
			InvState::Items => vec![&self.inv_area, &self.hotbar_area],
			InvState::All => vec![&self.inv_area, &self.armor_area, &self.hotbar_area],
			InvState::None => vec![],
		}
	}
}

impl UIManager {
	pub fn setup_inventory_ui(&mut self) {
		self.clear_elements();
		let inv_lay = ptr::get_gamestate().player_mut().inventory_mut();

		if let UIState::Inventory(state) = self.state.clone() {
			let layout = match state {
				InventoryUIState::Player { inv } => {
					InventoryLayout::calculate_for_player(inv, inv_lay)
				}
				InventoryUIState::Storage { inv, size } => {
					InventoryLayout::calculate_for_storage(size, inv, inv_lay)
				}
				InventoryUIState::Crafting { inv, size, result } => {
					InventoryLayout::calculate_for_crafting(size, result, inv, inv_lay)
				}
			};
			inv_lay.set_layout(&layout);
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
		let version_y = layout.panel_position.1 - version_height - PADDING;
		
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
				let (x, y) = area.get_slot_position(row, col);
				
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