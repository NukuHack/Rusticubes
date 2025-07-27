


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensure each variant has a u8 representation
pub enum Material {
	Iron,
	Gold,
	Diamond,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensure each variant has a u8 representation
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
impl MaterialLevel {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Hay => "Hay",
			Self::Wax => "Wax",
			Self::Talc => "Talc",
			Self::Gypsum => "Gypsum",
			Self::Ice => "Ice",
			Self::Calcite => "Calcite",
			Self::Coal => "Coal",
			Self::Fluorite => "Fluorite",
			Self::Obsidian => "Obsidian",
			Self::Apatite => "Apatite",
			Self::Iron => "Iron",
			Self::Orthoclase => "Orthoclase",
			Self::Quartz => "Quartz",
			Self::Steel => "Steel",
			Self::Topaz => "Topaz",
			Self::Titanium => "Titanium",
			Self::Emerald => "Emerald",
			Self::Corundum => "Corundum",
			Self::Tungsten => "Tungsten carbide",
			Self::Diamond => "Diamond",
			Self::BoronNitride => "Boron Nitride",
			Self::Graphene => "Graphene",
			Self::Carbyne => "Carbyne",
			Self::Adamantium => "Adamantium",
		}
	}
}