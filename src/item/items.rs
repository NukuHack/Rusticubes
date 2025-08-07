
use crate::fs::rs;
use crate::item::item_lut::{ItemFlags, ItemComp};
use crate::hs::string::MutStr;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
	name: MutStr,
	pub stack: u32,  // Typically 1-64 like Minecraft but let it be 255 for extreme cases
	pub data: Option<Box<CustomData>>,  // Boxed to reduce size when None
}
impl ItemStack {
    // More focused helper function
    #[inline] fn get_resources_and_target(&self) -> (Vec<String>, String) {
        let item_data = self.lut();
        // Get the reliable assets vector based on whether this is a block or item
        let resource_type = if item_data.is_block() { "block" } else { "item" };
        let resources = rs::find_png_resources(resource_type);
        // Construct the expected resource path
        let target_name = format!("{}/{}.png", resource_type, item_data.name);
        (resources, target_name)
    }
	pub fn to_icon(&self) -> String {
        let (resources, target_name) = self.get_resources_and_target();

		resources.iter()
			.find(|&res| *res == target_name)
			.cloned()
			.unwrap_or_else(|| {
				println!("No icon found for {}", self.name);
				resources.first()
					.cloned()
					.expect("Resources array should never be empty")
			})
	}
    #[inline] pub fn get_index(&self) -> Option<usize> {
        let (resources, target_name) = self.get_resources_and_target();
        // Find the index in the resources vector
        resources.iter().position(|res| *res == target_name)
    }

	#[inline] pub fn lut(&self) -> ItemComp {
		get_item_lut_ref().get(self.name.to_str()).unwrap_or(&DEFAULT_ITEM_COMP).clone()
	}
	#[inline] pub fn lut_idx(idx: usize) -> ItemComp {
		if let Some((_key, value)) = get_item_lut_ref().iter().nth(idx) {
			value.clone()
		} else {
			DEFAULT_ITEM_COMP.clone()
		}
	}
	#[inline] pub fn max_stack_size(&self) -> u32 {
		self.lut().max_stack
	}

	#[inline] pub const fn default() -> Self {
		Self::new("brick_grey")
	}
	#[inline] pub const fn new(name: &'static str) -> Self {
		Self::new_i(MutStr::from_str(name))
	}
	#[inline] pub const fn new_i(name: MutStr) -> Self {
		Self { name, stack: 64u32, data: None }
	}
	
	#[inline] pub fn is_block(&self) -> bool { matches!(self.lut().is_block(), true) }

	#[inline] pub fn get_block_id(&self) -> Option<u16> {
		if self.is_block() {
			return self.get_index().map(|id| id as u16);
		} None
	}

	#[inline] pub const fn with_stack(mut self, stack: u32) -> Self { self.stack = stack; self }
	#[inline] pub const fn set_stack(&mut self, stack: u32) { self.stack = stack }
	#[inline] pub fn stack(&self) -> u32 {
		if self.stack > self.max_stack_size()
		{ self.max_stack_size() }
		else { self.stack }
	}

	/// Checks if this item can be stacked with another
	pub fn can_stack_with(&self, other: &Self) -> bool {
		// Implement your stacking logic (same type, same metadata, etc.)
		self.name == other.name && self.data == other.data &&
		self.stack < self.max_stack_size()
	}

	/// Adds stack to this stack, returning any overflow
	pub fn add_stack(&mut self, amount: u32) -> u32 {
		let max_add = self.max_stack_size() - self.stack();
		let to_add = amount.min(max_add);
		self.set_stack(self.stack + to_add);
		amount - to_add
	}
	pub fn rem_stack(&mut self, amount: u32) -> u32 {
		let max_rem = self.stack();
		let to_rem = amount.min(max_rem);
		self.set_stack(self.stack + to_rem);
		amount - to_rem
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomData {
	pub name: Option<MutStr>,
	pub durability: Option<u16>,
	//pub effects -  // should be reworked later but the stuff what in minecraft gives + health and stuff
	//pub cosmetics - // stuff that would like make the color change or make the sword be double edged ...
}
impl CustomData {
	#[inline] pub const fn default() -> Self {
		Self {
			name: None,
			durability: None,
		}
	}
}


/// I implemented a purple-pink-black "0" block what would represent the "error" .. id 1 is air

pub const DEFAULT_ITEM_COMP: ItemComp = const { ItemComp::error().as_block() };

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
// Global mutable state with RwLock
static ITEM_LUT: OnceLock<RwLock<HashMap<String, ItemComp>>> = OnceLock::new();

#[inline] pub fn get_item_lut() -> &'static RwLock<HashMap<String, ItemComp>> {
	ITEM_LUT.get_or_init(|| RwLock::new(HashMap::new()))
}
#[inline] pub fn get_item_lut_mut() -> RwLockWriteGuard<'static, HashMap<String, ItemComp>> {
	get_item_lut().write().unwrap()
}
#[inline] pub fn get_item_lut_ref() -> RwLockReadGuard<'static, HashMap<String, ItemComp>> {
	get_item_lut().read().unwrap()
}

pub fn init_item_lut() {
	// Insert items (write lock)
	{
		let mut map = get_item_lut_mut();
		map.insert("air".to_string(), ItemComp::new("air").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK)));
		map.insert("brick_grey".to_string(), ItemComp::new("brick_grey").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK)));
		map.insert("brick_red".to_string(), ItemComp::new("brick_red").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK)));
		map.insert("bush".to_string(), ItemComp::new("bush").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK)));
		map.insert("wheat".to_string(), ItemComp::new("wheat").with_flag(ItemFlags::new(ItemFlags::IS_CONSUMABLE)));
		map.insert("iron_sword".to_string(), ItemComp::new("iron_sword").with_flag(ItemFlags::new(ItemFlags::IS_TOOL)));
		map.insert("bow".to_string(), ItemComp::new("bow").with_flag(ItemFlags::new(ItemFlags::IS_TOOL)));
		map.insert("arrow".to_string(), ItemComp::new("arrow"));
	}
	/*
	// Read items (read lock)
	{
		let map = get_item_lut_ref();
		println!("{:?}", map.get("air"));
	}
	*/
}
