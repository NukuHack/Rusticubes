
use std::num::NonZeroU16;
use crate::fs::rs;
use crate::item::item_lut::ItemComp;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
	name: String,
	pub stack: u32,  // Typically 1-64 like Minecraft but let it be 255 for extreme cases
	pub data: Option<Box<CustomData>>,  // Boxed to reduce size when None
}
impl ItemStack {
	///////////////////////////////
	// Construction and Initialization
	///////////////////////////////
	
	#[inline] 
	pub fn default() -> Self {
		Self::from_str("brick_grey")
	}
	
	#[inline] 
	pub fn new(name: String) -> Self {
		let stack = lut_by_name(&name).max_stack;
		Self { name, stack, data: None }
	}
	
	#[inline] 
	pub fn from_str(name: &'static str) -> Self {
		Self::new(name.to_string())
	}
	
	#[inline] 
	pub fn create(name: String, stack: u32, data: Option<Box<CustomData>>) -> Self {
		Self {name, stack, data}
	}
	
	/// Creates an ItemStack from a resource index where the first bit indicates if it's a block (1) or item (0)
	pub fn from_idx(resource_idx: usize) -> Self {
		// Extract the is_block flag from the first bit
		let is_block = (resource_idx & 1) == 1;
		// The actual index is the remaining bits
		let actual_idx = resource_idx >> 1;
		
		let resource_type = if is_block { "block" } else { "item" };
		let resources = rs::find_png_resources(resource_type);
		
		// Get the resource name or fall back to default if index is out of bounds
		let resource_name = resources.get(actual_idx)
			.map(|s| s.trim_start_matches(&format!("{}/", resource_type)).trim_end_matches(".png"))
			.unwrap_or("0");
		
		Self::new(resource_name.to_string())
	}

	///////////////////////////////
	// Property Accessors
	///////////////////////////////
	
	#[inline] 
	pub fn name(&self) -> &str { &self.name }
	
	#[inline] 
	pub fn stack(&self) -> u32 { self.max_stack_size().min(self.stack) }
	
	#[inline] 
	pub fn max_stack_size(&self) -> u32 {
		self.lut().max_stack
	}
	
	///////////////////////////////
	// Type Predicates
	///////////////////////////////
	
	#[inline] 
	pub fn is_block(&self) -> bool { self.lut().is_block() }
	
	#[inline] 
	pub fn is_armor(&self) -> bool { self.lut().is_armor() }
	
	#[inline] 
	pub fn is_tool(&self) -> bool { self.lut().is_tool() }
	
	#[inline] 
	pub fn is_weapon(&self) -> bool { self.lut().is_weapon() }

	#[inline] 
	pub fn is_storage(&self) -> bool { self.lut().is_storage() }
	
	#[inline] 
	pub fn is_consumable(&self) -> bool { self.lut().is_consumable() }

	///////////////////////////////
	// Stack Manipulation (Mutable)
	///////////////////////////////
	
	/// Sets the stack size to a specific value
	#[inline] 
	pub fn set_stack_size(&mut self, size: u32) { 
		self.stack = size; 
	}
	
	/// Sets the stack data
	#[inline] 
	pub fn set_stack_data(&mut self, data: Option<Box<CustomData>>) { 
		self.data = data; 
	}
	
	/// Sets the stack size to its maximum
	#[inline] 
	pub fn set_to_max_stack(&mut self) { 
		self.stack = self.max_stack_size(); 
	}
	
	/// Adds to the stack, returning any overflow amount
	pub fn add_to_stack(&mut self, amount: u32) -> u32 {
		let max_add = self.max_stack_size() - self.stack();
		let to_add = amount.min(max_add);
		self.stack += to_add;
		amount - to_add
	}
	
	/// Removes from the stack, returning the new stack if successful
	pub fn remove_from_stack(mut self, amount: u32) -> Option<Self> {
		if amount >= self.stack { 
			return None; 
		}
		self.stack -= amount;
		Some(self)
	}

	/// Adds to the stack, returning any overflow amount
	/// Removes from the stack, returning the new stack if successful
	pub fn stack_op(mut self, amount: i32) -> Option<(Self, Option<u32>)> {
		if amount > 0 {
			let rem = self.add_to_stack(amount as u32);
			return Some((self,Some(rem)));
		} else if amount < 0 {
			let item = self.remove_from_stack((-amount) as u32)?;
			Some((item, None))
		} else {
			Some((self, None))
		}
	}

	///////////////////////////////
	// Stack Operations (Immutable)
	///////////////////////////////
	
	/// Returns a new stack with the given size
	#[inline] 
	pub fn with_stack_size(self, size: u32) -> Self { 
		Self { stack: size, ..self }
	}
	
	/// Returns half of the current stack (rounded down)
	#[inline] 
	pub fn half_stack(&self) -> u32 { 
		(self.stack / 2).min(self.max_stack_size()) 
	}
	
	/// Splits the stack into two, modifying self and returning the split half (the bigger half, if size is odd)
	pub fn split_stack(&mut self) -> Option<Self> {
		let half = self.half_stack();
		let result = self.clone();
		self.set_stack_size(half);
		result.remove_from_stack(half)
	}

	///////////////////////////////
	// Conversion and Utility
	///////////////////////////////
	
	/// Converts to Option, returning None if stack is empty
	#[inline] 
	pub fn opt(self) -> Option<Self> { 
		if self.stack == 0 { None } else { Some(self) } 
	}

	#[inline] 
	pub fn can_stack_with(&self, other: &Self) -> bool {
		self.name == other.name && self.data == other.data
	}
	
	/// Gets the icon path for this item
	pub fn icon_path(&self) -> String {
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
	
	/// Gets the index of this item in the resources list
	#[inline]
	pub fn resource_index(&self) -> usize {
		let (resources, target_name) = self.get_resources_and_target();
		let Some(idx) = resources.iter().position(|res| *res == target_name) else { return 0; };
		let is_block_bit = if self.is_block() { 1 } else { 0 };
		(idx << 1) | is_block_bit
	}

	///////////////////////////////
	// Internal Helpers
	///////////////////////////////
	
	#[inline] 
	fn get_resources_and_target(&self) -> (Vec<String>, String) {
		let item_data = self.lut();
		let resource_type = if item_data.is_block() { "block" } else { "item" };
		let resources = rs::find_png_resources(resource_type);
		let target_name = format!("{}/{}.png", resource_type, item_data.name);
		(resources, target_name)
	}
	
	#[inline] fn lut(&self) -> ItemComp {
		lut_by_name(&self.name)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomData {
	pub name: Option<String>,
	pub durability: Option<NonZeroU16>,
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

#[inline] pub fn lut_by_name(name: &str) -> ItemComp {
	item_lut_ref().get(name).unwrap_or(&DEFAULT_ITEM_COMP).clone()
}


use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard};

// Global mutable state with RwLock
static ITEM_LUT: OnceLock<RwLock<HashMap<String, ItemComp>>> = OnceLock::new();

/// Returns a raw pointer to the OnceLock's storage location.
/// This is safe because it's just an address calculation.
pub const fn item_lut_ptr() -> *const OnceLock<RwLock<HashMap<String, ItemComp>>> {
	&ITEM_LUT as *const _
}

/// Returns a reference to the global item lookup table.
/// # Panics
/// Panics if it hasn't been initialized yet.
#[inline]
fn item_lut() -> &'static RwLock<HashMap<String, ItemComp>> {
	ITEM_LUT.get().expect("ItemLut should be initialized")
}

/// Returns an immutable guard for reading from the item lookup table.
///
/// # Panics
/// Panics if the lock is poisoned or cannot be acquired.
#[inline]
pub fn item_lut_ref() -> RwLockReadGuard<'static, HashMap<String, ItemComp>> {
	item_lut().read().expect("Failed to acquire read lock for item LUT")
}

pub fn clean_item_lut() {
	item_lut().write().expect("Failed to acquire write lock for item LUT").clear();
}

pub fn print_all_items() {
	let registry = item_lut_ref();
	println!("Registered items:");
	let blocks = rs::find_png_resources("block");
	let items = rs::find_png_resources("item");
	for (name, item_data) in registry.iter() {
		let idx:usize = if item_data.is_block() {
			let target_name = format!("{}/{}.png", "block", item_data.name);
			blocks.iter().position(|res| *res == target_name).unwrap_or(0)
		} else {
			let target_name = format!("{}/{}.png", "item", item_data.name);
			items.iter().position(|res| *res == target_name).unwrap_or(0)
		};

		println!("- {:?} ; {:?} ; {:?}", item_data, name, idx);
	}
}

pub fn init_item_lut() {
	use crate::item::item_lut::{ArmorData, ToolData, };
	use crate::item::material::{ArmorType, ToolType, MaterialLevel};
	
	{
		// idk how to only specify the init and not get_or_init so yeah
		let _ = ITEM_LUT.get_or_init(|| RwLock::new(HashMap::new()));
	}
	// Insert items (write lock)
	{
		let mut map = item_lut().write().expect("Failed to acquire write lock for item LUT");
		map.insert("air".to_string(), ItemComp::new("air").as_block());
		map.insert("brick_grey".to_string(), ItemComp::new("brick_grey").as_block());
		map.insert("brick_red".to_string(), ItemComp::new("brick_red").as_block());
		map.insert("bush".to_string(), ItemComp::new("bush").as_block());
		map.insert("wheat".to_string(), ItemComp::new("wheat").as_consumable());
		map.insert("iron_sword".to_string(), ItemComp::new("iron_sword").as_tool(ToolData::Single{ equip_type:ToolType::String, tier: MaterialLevel::Calcite }).with_damage(5).with_stack(1));
		map.insert("bow".to_string(), ItemComp::new("bow").with_stack(1));
		map.insert("arrow".to_string(), ItemComp::new("arrow"));
		map.insert("plank".to_string(), ItemComp::new("plank").as_block().as_storage((5,9).into()));
		map.insert("coat".to_string(), ItemComp::new("coat").as_armor(ArmorData::Single{ equip_type:ArmorType::Torso, tier: MaterialLevel::Calcite }).with_stack(1));
		map.insert("crafting".to_string(), ItemComp::new("crafting").as_block().as_storage((3,3).into()));
	}
}
