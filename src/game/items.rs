
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: u32,  // Typically 1-64 like Minecraft
    pub data: Option<Box<ItemData>>,  // Boxed to reduce size when None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Block(BlockId),
    Item(ItemId),
}


// Use newtype pattern for better type safety and documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

// Use bitflags for extensible tool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolFlags(pub u8);
impl ToolFlags {
    const PICKAXE:u8 = 1 << 0;
    const AXE:u8 = 1 << 1;
    const SHOVEL:u8 = 1 << 2;
    const HOE:u8 = 1 << 3;
    const SWORD:u8 = 1 << 4;
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

// Implement Default where it makes sense
impl Default for ItemStack {
    fn default() -> Self {
        Self {
            item: Item::Item(ItemId(0)),
            quantity: 1,
            data: None,
        }
    }
}

// Add convenience methods
impl ItemStack {
    pub fn new_block(block: BlockId, quantity: u32) -> Self {
        Self {
            item: Item::Block(block),
            quantity,
            data: None,
        }
    }
    
    pub fn new_item(item: ItemId, quantity: u32) -> Self {
        Self {
            item: Item::Item(item),
            quantity,
            data: None,
        }
    }
    
    pub fn is_block(&self) -> bool {
        matches!(self.item, Item::Block(_))
    }
    
    pub fn is_item(&self) -> bool {
        matches!(self.item, Item::Item(_))
    }
}