
use crate::get_nth_file;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
	pub item: Item,
	pub quantity: u8,  // Typically 1-64 like Minecraft
	pub data: Option<Box<ItemData>>,  // Boxed to reduce size when None
}

const EXTRA_BLOCK_DATA_OFFSET:usize = 1usize; // currently only a single one : air 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
	Block(u16),
	Item(u16),
}
impl Item {
	pub fn to_icon(&self) -> String {
		let file_path = match self {
			Self::Block(id) => {
				get_nth_file!(*id as usize - EXTRA_BLOCK_DATA_OFFSET, "blocks")
			}
			Self::Item(id) => {
				get_nth_file!(*id as usize - EXTRA_BLOCK_DATA_OFFSET, "items") // currently crashes ... allwaysS
			}
		};
		return file_path.to_string_lossy().into_owned();
	}
}


// Use bitflags for extensible tool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolFlags(pub u8);
#[allow(dead_code)]
impl ToolFlags {
	const PICKAXE:u8 = 1 << 0; // stone related thing
	const AXE:u8 = 1 << 1; // wood related thing
	const SHOVEL:u8 = 1 << 2; // dirt related thing
	const HOE:u8 = 1 << 3; // leaf related thing
	const SWORD:u8 = 1 << 4; // web related thing
	const SCISSORS:u8 = 1 << 5; // wool related thing
	// Add more as needed
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemData {
	pub durability: Option<u16>,
	pub tool: Option<ToolFlags>,
	pub hunger: Option<i8>,
	pub armor: Option<i8>,
	pub effects: Option<Vec<u32>>, // might be reworked later but the stuff what in minecraft gives + health and stuff
}

impl ItemStack {
	#[inline] pub const fn default() -> Self {
		Self {
			item: Item::Item(0u16),
			quantity: 1u8,
			data: None,
		}
	}

	#[inline] pub const fn new_block(block: u16, quantity: u8) -> Self {
		Self {
			item: Item::Block(block),
			quantity,
			data: None,
		}
	}
	
	#[inline] pub const fn new_item(item: u16, quantity: u8) -> Self {
		Self {
			item: Item::Item(item),
			quantity,
			data: None,
		}
	}
	
	#[inline] pub const fn is_block(&self) -> bool {
		matches!(self.item, Item::Block(_))
	}

	#[inline] pub const fn get_block_id(&self) -> Option<u16> {
		if let Item::Block(idx) = self.item {
			return Some(idx);
		} None
	}
	
	#[inline] pub const fn is_item(&self) -> bool {
		matches!(self.item, Item::Item(_))
	}
}