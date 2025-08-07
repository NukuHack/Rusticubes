
use crate::fs::rs;
use crate::item::item_lut::{ItemFlags, ItemComp};
use crate::hs::string::MutStr;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u16);
impl ItemId {
	#[inline] pub const fn inner(&self) -> u16 {
		self.0
	}
	#[inline] pub fn from(val:u16) -> Self {
		Self(val)
	}
	// New method to get the name from the LUT
	#[inline] pub fn name(&self) -> &'static str {
	  ITEM_REGISTRY_LUT[self.inner() as usize].name
	}
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
	pub id: ItemId,
	pub stack: u32,  // Typically 1-64 like Minecraft but let it be 255 for extreme cases
	pub data: Option<Box<CustomData>>,  // Boxed to reduce size when None
}
impl ItemStack {
	pub fn to_icon(&self) -> String {
		let item_data = self.lut();
		let resource_type = if item_data.is_block() { "block" } else { "item" };

		let resources = rs::find_png_resources(resource_type);
		let target_name = format!("{}/{}.png", resource_type, item_data.name);

		resources.iter()
			.find(|&res| *res == target_name)
			.cloned()
			.unwrap_or_else(|| {
				println!(
					"No icon found for {} '{}' (ID: {}).",
					resource_type,
					item_data.name,
					self.id.inner(),
				);
				resources.first()
					.cloned()
					.expect("Resources array should never be empty")
			})
	}
	#[inline] pub fn lut(&self) -> ItemComp {
		ITEM_REGISTRY_LUT[self.id.inner() as usize].copy()
	}
	#[inline] pub fn max_stack_size(&self) -> u32 {
		self.lut().max_stack
	}

	#[inline] pub const fn default() -> Self {
		Self::new(1)
	}
	#[inline] pub const fn new(id: u16) -> Self {
		Self::new_i(ItemId(id))
	}
	#[inline] pub const fn new_i(id: ItemId) -> Self {
		Self { id, stack: 64u32, data: None }
	}
    // New constructor that takes a name instead of ID
    #[inline] pub const fn new_n(name: &'static str) -> Self {
        Self::new_i(ItemId::from_str(name))
    }
	
	#[inline] pub fn is_block(&self) -> bool { matches!(self.lut().is_block(), true) }

	#[inline] pub fn get_block_id(&self) -> Option<ItemId> {
		if self.is_block() {
			return Some(self.id);
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
		self.id == other.id && self.data == other.data &&
		self.stack < self.max_stack_size()
	}

	/// Adds stack to this stack, returning any overflow
	pub fn add_stack(&mut self, amount: u32) -> u32 {
		let max_add = self.max_stack_size() - self.stack();
		let to_add = amount.min(max_add);
		self.set_stack(self.stack + to_add);
		amount - to_add
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


///Map size is bigger than needed, but this results a lot of unused space,
///Ofc you can not modify the array at runtime so you have to make it as big as it needs to be at compile-time for sure
/// for this reason i implemented a purple-pink-black "0" block what would represent the "error" .. id 1 is air

const MAP_SIZE: usize = 100usize;

pub const ITEM_REGISTRY_LUT: [ItemComp; MAP_SIZE] = {
	let mut map = [const { ItemComp::error().as_block() }; MAP_SIZE];
	
	map[1] = ItemComp::new_i(ItemId::from_str("air"), "air").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK));
	map[2] = ItemComp::new_i(ItemId::from_str("brick_grey"), "brick_grey").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK));
	map[3] = ItemComp::new_i(ItemId::from_str("brick_red"), "brick_red").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK));
	map[4] = ItemComp::new_i(ItemId::from_str("bush"), "bush").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK));
	map[5] = ItemComp::new_i(ItemId::from_str("wheat"), "wheat").with_flag(ItemFlags::new(ItemFlags::IS_CONSUMABLE));
	map[6] = ItemComp::new_i(ItemId::from_str("iron_sword"), "iron_sword").with_flag(ItemFlags::new(ItemFlags::IS_TOOL));
	map[7] = ItemComp::new_i(ItemId::from_str("bow"), "bow").with_flag(ItemFlags::new(ItemFlags::IS_TOOL));
	map[8] = ItemComp::new_i(ItemId::from_str("arrow"), "arrow");
	// add more
	
	map
};

impl ItemId {
	pub const fn from_str(string: &'static str) -> Self {
		const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
			if a.len() != b.len() {
				return false;
			}
			let mut i = 0;
			while i < a.len() {
				if a[i] != b[i] {
					return false;
				}
				i += 1;
			}
			true
		}
		let bytes = string.as_bytes();

		if bytes_eq(bytes, b"0") {
			return Self(0);
		} else if bytes_eq(bytes, b"air") {
			return Self(1);
		} else if bytes_eq(bytes, b"brick_grey") {
			return Self(2);
		} else if bytes_eq(bytes, b"brick_red") {
			return Self(3);
		} else if bytes_eq(bytes, b"bush") {
			return Self(4);
		} else if bytes_eq(bytes, b"wheat") {
			return Self(5);
		} else if bytes_eq(bytes, b"iron_sword") {
			return Self(6);
		} else if bytes_eq(bytes, b"bow") {
			return Self(7);
		} else if bytes_eq(bytes, b"arrow") {
			return Self(8);
		}
		// add more
		
		Self(0)
	}
}
