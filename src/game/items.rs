
use crate::fs::rs;
use crate::game::item_lut::ItemComp;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(u16);
impl ItemId {
	pub fn inner(&self) -> u16 {
		self.0
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
	pub id: ItemId,
	pub quantity: u8,  // Typically 1-64 like Minecraft but let it be 255 for extreme cases
	pub data: Box<CustomData>,  // Boxed to reduce size when None
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

const EXTRA_BLOCK_DATA_OFFSET:usize = 1usize; // currently only a single one : air 

impl ItemStack {
	pub fn to_icon(&self) -> String {
		let resources = if self.lut().is_block() {
			rs::find_png_resources("block")
		} else {
			rs::find_png_resources("item")
		};
		
		resources.get(self.id.0 as usize - EXTRA_BLOCK_DATA_OFFSET).cloned().unwrap()
	}
	pub fn lut(&self) -> ItemComp {
		// yes not yet implemented ...
		// will get the data from a LUT so it should be fast ... i hope 
		todo!();
	}

	#[inline] pub fn default() -> Self {
		Self {
			id: ItemId(0u16),
			quantity: 1u8,
			data: Box::new(CustomData::default()),
		}
	}

	#[inline] pub fn new_block(id: u16, quantity: u8) -> Self {
		Self {
			id: ItemId(id),
			quantity,
			data: Box::new(CustomData::default()),
		}
	}
	
	#[inline] pub fn new_item(id: u16, quantity: u8) -> Self {
		Self {
			id: ItemId(id),
			quantity,
			data: Box::new(CustomData::default()),
		}
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

