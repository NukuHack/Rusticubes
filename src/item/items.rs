
use crate::fs::rs;
use crate::item::item_lut::ItemComp;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u16);
impl ItemId {
	pub const fn inner(&self) -> u16 {
		self.0
	}
	pub fn from(val:u16) -> Self {
		Self(val)
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
	pub const fn lut(&self) -> ItemComp {
		// yes not yet implemented ...
		// will get the data from a LUT so it should be fast ... i hope 
		ITEM_REGISTRY_LUT[self.id.inner() as usize].copy()
	}
	pub const fn max_stack_size(&self) -> u32 {
		self.lut().max_stack
	}

	#[inline] pub const fn default() -> Self {
		Self::new(1)
	}
	#[inline] pub const fn new(id: u16) -> Self {
		Self::new_i(ItemId(id))
	}
	#[inline] pub const fn new_i(id: ItemId) -> Self {
		Self { id, stack: 1u32, data: None }
	}
	
	#[inline] pub fn is_block(&self) -> bool { matches!(self.lut().is_block(), true) }
	#[inline] pub fn is_item(&self) -> bool { !self.is_block() }

	#[inline] pub fn get_block_id(&self) -> Option<ItemId> {
		if self.lut().is_block() {
			return Some(self.id);
		} None
	}

	#[inline] pub const fn with_stack(mut self, stack: u32) -> Self { self.stack = stack; self }
	#[inline] pub const fn set_stack(&mut self, stack: u32) { self.stack = stack }
	#[inline] pub const fn stack(&self) -> u32 {
		let q = self.stack;
		if q > self.max_stack_size() { self.max_stack_size() } else { q }
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
	pub name: Option<&'static str>,
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


const MAP_SIZE: usize = 100usize;
pub const ITEM_REGISTRY_LUT: [ItemComp; MAP_SIZE] = generate_item_registry_lut();
///Map size is bigger than needed, but this results a lot of unused space,
///Ofc you can not modify the array at runtime so you have to make it as big as it needs to be at compile-time for sure
/// for this reason i implemented a purple-pink-black "0" block what would represent the "error" .. id 1 is air

macro_rules! generate_item_registry {
	($($id:literal => $name:literal - $block:expr),* $(,)?) => {
		pub const fn generate_item_registry_lut() -> [ItemComp; MAP_SIZE] {
			let mut map = [const { ItemComp::error().as_block() }; MAP_SIZE];
			
			$(
				map[$id] = if $block {
					ItemComp::new($id, $name).as_block()
				} else { ItemComp::new($id, $name) };
			)*
			
			map
		}
		
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
				if bytes_eq(bytes, "0".as_bytes()) {
					Self(0)
				}
				$(
					else if bytes_eq(bytes, $name.as_bytes()) {
						Self($id)
					}
				)*
				else {
					Self(0)
				}
			}
		}
	};
}

// Usage:
generate_item_registry! {
	// 0 is the error, but used inside the macro so not mapped here but only inside -> directly
	1 => "air" - true,
	2 => "brick_grey" - true,
	3 => "brick_red" - true,
	4 => "bush" - true,
	5 => "bread" - false,
	// Add more items here...
}