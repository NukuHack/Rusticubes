
use crate::fs::rs;
use crate::game::item_lut::ItemComp;


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
	pub quantity: u32,  // Typically 1-64 like Minecraft but let it be 255 for extreme cases
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

	#[inline] pub const fn default() -> Self {
		Self::new(1)
	}
	#[inline] pub const fn new(id: u16) -> Self {
		Self::new_i(ItemId(id))
	}
	#[inline] pub const fn new_i(id: ItemId) -> Self {
		Self {
			id,
			quantity: 1u32,
			data: None
		}
	}
	#[inline] pub const fn quantity(mut self, quantity: u32) -> Self {
		self.quantity = quantity;
		self
	}
	
	#[inline] pub fn is_block(&self) -> bool {
		matches!(self.lut().is_block(), true)
	}

	#[inline] pub fn get_block_id(&self) -> Option<ItemId> {
		if self.lut().is_block() {
			return Some(self.id);
		} None
	}
	
	#[inline] pub fn is_item(&self) -> bool {
		!self.is_block()
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
	($($id:literal => $name:literal),* $(,)?) => {
		pub const fn generate_item_registry_lut() -> [ItemComp; MAP_SIZE] {
			let mut map = [const { ItemComp::error().as_block() }; MAP_SIZE];
			
			$(
				map[$id] = ItemComp::new($id, $name).as_block();
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
	1 => "air",
	2 => "brick_grey",
	3 => "brick_red",
	4 => "bush",
	// Add more items here...
}