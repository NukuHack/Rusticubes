
use crate::game::material::MaterialLevel;
use crate::game::items::ItemId;

use ahash::AHasher;
use std::{
    collections::HashMap,
    hash::BuildHasherDefault,
};

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

// Global registry that's initialized at program start
struct ItemRegistry {
    components: FastMap<ItemId, &'static ItemComp>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self {
           components: FastMap::with_capacity_and_hasher(100, BuildHasherDefault::<AHasher>::default())
        }
    }
    // Called during initialization
    pub fn register(&mut self, id: ItemId, component: &'static ItemComp) {
        self.components.insert(id, component);
    }
    
    // Runtime access
    pub fn get_component(&self, id: ItemId) -> Option<&'static ItemComp> {
        self.components.get(&id).copied()
    }
}

pub struct ItemComp {
	pub id: ItemId,
	pub name: &'static str,
	pub max_stack: u32,
	pub is_block: bool,
	pub data: Option<Box<ItemData>>,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemData {
	pub max_durability: Option<std::num::NonZeroU32>,
	pub tool: ToolData,
	pub equipent: ArmorData,
	pub hunger: Option<i32>, // -hunger is actually food, that is what decreases hunger
	pub armor: Option<i32>, // -armor is weakening? not sure if that would be used too much tho
	pub damage: Option<i32>, // -damage would heal? might make this u32
	pub speed: Option<i32>, // -speed would be slowdown, that makes the weapon harder (slower) to swing
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolData {
    None,
    Single(ToolType, MaterialLevel),
    Multiple(ToolSet),
}

impl ToolData {
    #[inline] pub fn is_tool(&self) -> bool {
        !matches!(self, Self::None)
    }
    
    #[inline] pub fn get_tier(&self, tool_type: ToolType) -> Option<MaterialLevel> {
        match self {
            Self::None => None,
            Self::Single(t, tier) if *t == tool_type => Some(*tier),
            Self::Single(_, _) => None,
            Self::Multiple(set) => set.get_tier(tool_type),
        }
    }
    
    #[inline] pub fn as_single(&self) -> Option<(ToolType, MaterialLevel)> {
        if let Self::Single(t, tier) = self {
            Some((*t, *tier))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArmorData {
    None,
    Single(ArmorType, MaterialLevel),
    Multiple(ArmorSet),
}

impl ArmorData {
    #[inline] pub fn is_armor(&self) -> bool {
        !matches!(self, Self::None)
    }
    
    #[inline] pub fn get_tier(&self, armor_type: ArmorType) -> Option<MaterialLevel> {
        match self {
            Self::None => None,
            Self::Single(t, tier) if *t == armor_type => Some(*tier),
            Self::Single(_, _) => None,
            Self::Multiple(set) => set.get_tier(armor_type),
        }
    }
    
    #[inline] pub fn as_single(&self) -> Option<(ArmorType, MaterialLevel)> {
        if let Self::Single(t, tier) = self {
            Some((*t, *tier))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensure each tool has a u8 (0-255) representation 
pub enum ToolType {
    Stone,  // pickaxe related thing
    Wood,   // axe related thing
    Dirt,   // shovel related thing
    Crop,   // hoe related thing
    String, // sword and scissors related thing
    Metal,  // strong pickaxe related thing
    // Add up to 2 more tool types as needed (max 8 total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolSet {
    // Bits 0-7: tool types (8 possible types)
    types: u8,
    // Packed tiers for each possible tool type
    // Each tool type gets 8 bits (0-255) in the u64
    tiers: u64,
}

impl ToolSet {
    #[inline] pub fn new() -> Self {
        ToolSet {
            types: 0,
            tiers: 0,
        }
    }
    #[inline] pub fn add_tool(&mut self, tool_type: ToolType, tier: MaterialLevel) {
        if self.has_tool(tool_type) {
            panic!("has that tool type already");
        }
        let index = tool_type as u8;
        self.types |= 1 << index;
        self.set_tier(index, tier as u8);
    }
    #[inline] pub fn remove_tool(&mut self, tool_type: ToolType) {
        if !self.has_tool(tool_type) {
            return;
        }
        let index = tool_type as u8;
        self.types &= !(1 << index);
        self.set_tier(index, 0);
    }
    #[inline] pub fn has_tool(&self, tool_type: ToolType) -> bool {
        let index = tool_type as u8;
        (self.types & (1 << index)) != 0
    }

    #[inline] pub fn get_tier(&self, tool_type: ToolType) -> Option<MaterialLevel> {
        if self.has_tool(tool_type) {
            let index = tool_type as u8;
            let tier_byte = ((self.tiers >> (index * 8)) & 0xFF) as u8;
            Some(unsafe { std::mem::transmute(tier_byte) })
        } else {
            None
        }
    }

    #[inline] pub fn iter(&self) -> impl Iterator<Item = (ToolType, MaterialLevel)> + '_ {
        (0..8).filter_map(move |i| {
            if (self.types & (1 << i)) != 0 {
                let tier_byte = ((self.tiers >> (i * 8)) & 0xFF) as u8;
                Some((
                    unsafe { std::mem::transmute(i as u8) },
                    unsafe { std::mem::transmute(tier_byte) },
                ))
            } else {
                None
            }
        })
    }

    #[inline] fn set_tier(&mut self, index: u8, tier: u8) {
        let shift = index * 8;
        self.tiers &= !(0xFF << shift); // Clear the existing tier
        self.tiers |= (tier as u64) << shift; // Set the new tier
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensure each armor has a u8 (0-255) representation 
pub enum ArmorType {
    //Core Armor Slots (Typically Fixed Slots) 
    Head, // – Helmets, Hats, Crowns, Masks
    Torso, // – Chestplates, Robes, Tunics, Breastplates
    Legs, // – Greaves, Leggings, Pants, Skirts
    Feet, // – Boots, Sandals, Sabatons
    Arms, // – Pauldrons, Spaulders, Arm Guards
    Hands, // – Gauntlets, Gloves, Bracers
    Back, // – Cloaks, Capes, Wings, Backpacks
    Neck, // – Amulets, Necklaces, Pendants
    Finger, // – Rings (often allows 1-2 equipped)
    //Additional/Expanded Slots 
    Shoulders, // – Separate from Arms (common in games like WoW)
    Waist, // – Belts, Sashes, Girdles
    Eyes, // – Goggles, Glasses, Blindfolds
    Face, // – Masks, Veils (sometimes separate from Head)
    Aura, // – Cosmetic or buff-granting effects (e.g., "Holy Aura")
    Pocket, // – Utility items (e.g., Thieves' Tools, Quivers)
    // Add up to 1 more armor types as needed (max 16 total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorSet {
    // Bits 0-15: armor types (16 possible types)
    types: u16,
    // Packed tiers for each possible armor type
    // Each armor type gets 8 bits (0-255) in the u128
    tiers: u128,
}

impl ArmorSet {
    #[inline] pub fn new() -> Self {
        ArmorSet {
            types: 0,
            tiers: 0,
        }
    }
    
    #[inline] pub fn add_armor(&mut self, armor_type: ArmorType, tier: MaterialLevel) {
        if self.has_armor(armor_type) {
            panic!("has that armor type already");
        }
        let index = armor_type as u8;
        self.types |= 1 << index;
        self.set_tier(index, tier as u8);
    }
    
    #[inline] pub fn remove_armor(&mut self, armor_type: ArmorType) {
        if !self.has_armor(armor_type) {
            return;
        }
        let index = armor_type as u8;
        self.types &= !(1 << index);
        self.set_tier(index, 0);
    }
    
    #[inline] pub fn has_armor(&self, armor_type: ArmorType) -> bool {
        let index = armor_type as u8;
        (self.types & (1 << index)) != 0
    }

    #[inline] pub fn get_tier(&self, armor_type: ArmorType) -> Option<MaterialLevel> {
        if self.has_armor(armor_type) {
            let index = armor_type as u8;
            let tier_byte = ((self.tiers >> (index * 8)) & 0xFF) as u8;
            Some(unsafe { std::mem::transmute(tier_byte) })
        } else {
            None
        }
    }

    #[inline] pub fn iter(&self) -> impl Iterator<Item = (ArmorType, MaterialLevel)> + '_ {
        (0..16).filter_map(move |i| {
            if (self.types & (1 << i)) != 0 {
                let tier_byte = ((self.tiers >> (i * 8)) & 0xFF) as u8;
                Some((
                    unsafe { std::mem::transmute(i as u8) },
                    unsafe { std::mem::transmute(tier_byte) },
                ))
            } else {
                None
            }
        })
    }

    #[inline] fn set_tier(&mut self, index: u8, tier: u8) {
        let shift = index * 8;
        self.tiers &= !(0xFF << shift); // Clear the existing tier
        self.tiers |= (tier as u128) << shift; // Set the new tier
    }
}


