
use crate::game::items;
use crate::ui::inventory as inv;

pub const ARMOR: u8 = 4; // 4 slots by default
pub const HOTBAR: u8 = 5; // 5 slots by default
pub const INV_ROW: u8 = 3; // 3 rows by default -> 21 items
pub const INV_COL: u8 = 7; // 7 columns by default

#[derive(Clone, PartialEq)]
pub struct Inventory {
	armor: Vec<items::ItemStack>,
	armor_max: u8,
	inner: Vec<items::ItemStack>,
	inner_max: (u8,u8),
	hotbar: Vec<items::ItemStack>,
	hotbar_max: u8,

	pub layout: Option<inv::InventoryLayout>,
}
impl Default for Inventory {
	fn default() -> Self {
		Self{
			armor: Vec::new(),
			inner: Vec::new(),
			hotbar: Vec::new(),

			armor_max: ARMOR,
			hotbar_max: HOTBAR,
			inner_max: (INV_ROW,INV_COL),

			layout: None,
		}
	}
}

impl Inventory {
	pub fn armor(&self) -> u8 {
		self.armor_max
	}
	pub fn hotbar(&self) -> u8 {
		self.hotbar_max
	}
	pub fn inv_row(&self) -> u8 {
		self.inner_max.0
	}
	pub fn inv_col(&self) -> u8 {
		self.inner_max.1
	}

	pub fn set_layout(&mut self, layout: &inv::InventoryLayout) {
		self.layout = Some(layout.clone());
	}
}