


pub trait EquipmentType: Copy + Clone {
	const MAX_VARIANTS: u8;
	const TO_U8: fn(Self) -> u8;
}

impl EquipmentType for ToolType {
	const MAX_VARIANTS: u8 = 8;
	const TO_U8: fn(Self) -> u8 = |x| x as u8;
}

impl EquipmentType for ArmorType {
	const MAX_VARIANTS: u8 = 16;
	const TO_U8: fn(Self) -> u8 = |x| x as u8;
}

/*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensure each variant has a u8 representation, so up to 255 type
pub enum Material {
	Wood,
	Stone,
	Leather,
	Copper,
	Iron,
	WroughtIron,
	Gold,
	Steel,
	DamascusSteel,
	StainlessSteel,
	Titanium,
	Tungsten,
	Diamond,
	Neutronium,
}
impl Material {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Wood => "Wood",
			Self::Stone => "Stone",
			Self::Leather => "Leather",
			Self::Copper => "Copper",
			Self::Iron => "Iron",
			Self::WroughtIron => "WroughtIron",
			Self::Gold => "Gold",
			Self::Steel => "Steel",
			Self::DamascusSteel => "DamascusSteel",
			Self::StainlessSteel => "StainlessSteel",
			Self::Titanium => "Titanium",
			Self::Tungsten => "Tungsten",
			Self::Diamond => "Diamond",
			Self::Neutronium => "Neutronium",
		}
	}
	#[inline] pub const fn from_u8(value: u8) -> Option<Self> {
		unsafe { std::mem::transmute(value) }
	}
}
*/

pub trait BasicConversion<T> {
    const STRINGS: &'static [&'static str];
    
    fn to_u8(&self) -> u8;
    fn from_u8(value: u8) -> Option<Self> where Self: Sized;
    fn as_str(&self) -> &'static str;
    fn from_str(s: &str) -> Option<Self> where Self: Sized;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ToolType {
	Stone = 0,  // pickaxe related thing
	Wood = 1,   // axe related thing
	Dirt = 2,   // shovel related thing
	Crop = 3,   // hoe related thing
	String = 4, // sword and scissors related thing
	Metal = 5,  // strong pickaxe related thing
	// Add up to 2 more tool types as needed (max 8 total)
}
impl BasicConversion<Self> for ToolType {
    const STRINGS: &'static [&'static str] = &[
        "Stone",
        "Wood",
        "Dirt",
        "Crop",
        "String",
        "Metal",
    ];

    #[inline]
    fn to_u8(&self) -> u8 {
        *self as u8
    }

    #[inline]
    fn from_u8(value: u8) -> Option<Self> {
        if value < Self::STRINGS.len() as u8 {
            unsafe { Some(std::mem::transmute(value)) }
        } else {
            None
        }
    }

    fn as_str(&self) -> &'static str {
        Self::STRINGS[*self as usize]
    }

    fn from_str(s: &str) -> Option<Self> {
        Self::STRINGS
            .iter()
            .position(|&name| name == s)
            .and_then(|idx| Self::from_u8(idx as u8))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ArmorType {
	//Core Armor Slots (Typically Fixed Slots) 
	Head = 0, // – Helmets, Hats, Crowns, Masks
	Torso = 1, // – Chestplates, Robes, Tunics, Breastplates
	Legs = 2, // – Greaves, Leggings, Pants, Skirts
	Feet = 3, // – Boots, Sandals, Sabatons
	Arms = 4, // – Pauldrons, Spaulders, Arm Guards
	Hands = 5, // – Gauntlets, Gloves, Bracers
	Back = 6, // – Cloaks, Capes, Wings, Backpacks
	Neck = 7, // – Amulets, Necklaces, Pendants
	Finger = 8, // – Rings (often allows 1-2 equipped)
	//Additional/Expanded Slots 
	Shoulders = 9, // – Separate from Arms (common in games like WoW)
	Waist = 10, // – Belts, Sashes, Girdles
	Eyes = 11, // – Goggles, Glasses, Blindfolds
	Face = 12, // – Masks, Veils (sometimes separate from Head)
	Pocket = 13, // – Utility items (e.g., Thieves' Tools, Quivers)
	Aura = 14, // – Cosmetic or buff-granting effects (e.g., "Holy Aura")
	// Add up to 1 more armor types as needed (max 16 total)
}
impl BasicConversion<Self> for ArmorType {
    const STRINGS: &'static [&'static str] = &[
        "Head",
        "Torso",
        "Legs",
        "Feet",
        "Arms",
        "Hands",
        "Back",
        "Neck",
        "Finger",
        "Shoulders",
        "Waist",
        "Eyes",
        "Face",
        "Pocket",
        "Aura",
    ];

    #[inline]
    fn to_u8(&self) -> u8 {
        *self as u8
    }

    #[inline]
    fn from_u8(value: u8) -> Option<Self> {
        if value < Self::STRINGS.len() as u8 {
            unsafe { Some(std::mem::transmute(value)) }
        } else {
            None
        }
    }

    fn as_str(&self) -> &'static str {
        Self::STRINGS[*self as usize]
    }

    fn from_str(s: &str) -> Option<Self> {
        Self::STRINGS
            .iter()
            .position(|&name| name == s)
            .and_then(|idx| Self::from_u8(idx as u8))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensure each variant has a u8 representation, so up to 255 type
pub enum MaterialLevel {
	Hay,          // Weakest material, crumbles under pressure
	Wax,          // Soft, deforms at room temperature
	Talc,         // Talc (Scratched by a fingernail)
	Gypsum,       // Gypsum (Barely scratched by a fingernail)
	Ice,          // Brittle, fractures under slight force
	Calcite,      // Calcite (Scratched by a copper coin)
	Coal,         // Weak carbon structure, shatters easily
	Fluorite,     // Fluorite (Scratched by a knife)
	Obsidian,     // Volcanic glass, sharp but brittle
	Apatite,      // Apatite (Scratched by a knife with effort)
	Iron,         // Pure iron (Dents under heavy blows)
	Orthoclase,   // Orthoclase (Scratches glass)
	Quartz,       // Quartz (Scratches steel easily)
	Steel,        // Hardened steel (Resists abrasion)
	Topaz,        // Topaz (Harder than quartz)
	Titanium,     // Titanium alloy (High strength-to-weight)
	Emerald,      // Emerald (Tough but brittle gemstone)
	Corundum,     // Corundum (Sapphire/ruby, scratches steel)
	Tungsten,     // Tungsten carbide (Extreme hardness)
	Diamond,      // Diamond (Hardest natural material)
	BoronNitride, // Cubic boron nitride (Near-diamond hardness)
	Graphene,     // Graphene (Strongest known material)
	Carbyne,      // Carbyne (Theoretical, stronger than graphene)
	Adamantium,   // Fictional (Indestructible comic metal)
	// Add more tiers as needed (up to 255 total)
}
impl BasicConversion<Self> for MaterialLevel {
    const STRINGS: &'static [&'static str] = &[
        "Hay",
        "Wax",
        "Talc",
        "Gypsum",
        "Ice",
        "Calcite",
        "Coal",
        "Fluorite",
        "Obsidian",
        "Apatite",
        "Iron",
        "Orthoclase",
        "Quartz",
        "Steel",
        "Topaz",
        "Titanium",
        "Emerald",
        "Corundum",
        "Tungsten carbide",
        "Diamond",
        "Boron Nitride",
        "Graphene",
        "Carbyne",
        "Adamantium",
    ];

    #[inline]
    fn to_u8(&self) -> u8 {
        *self as u8
    }

    #[inline]
    fn from_u8(value: u8) -> Option<Self> {
        if value < Self::STRINGS.len() as u8 {
            unsafe { Some(std::mem::transmute(value)) }
        } else {
            None
        }
    }

    fn as_str(&self) -> &'static str {
        Self::STRINGS[*self as usize]
    }

    fn from_str(s: &str) -> Option<Self> {
        Self::STRINGS
            .iter()
            .position(|&name| name == s)
            .and_then(|idx| Self::from_u8(idx as u8))
    }
}
