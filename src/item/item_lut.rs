use crate::item::material::MaterialLevel;
use crate::item::items::ItemId;
use std::{
	num::NonZeroU32,
	cmp::PartialEq,
	marker::PhantomData,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemFlags(u32);
impl ItemFlags {
	pub const IS_BLOCK: u32 = 1 << 0;
	pub const IS_TOOL: u32 = 1 << 1;
	pub const IS_ARMOR: u32 = 1 << 2;
	pub const IS_CONSUMABLE: u32 = 1 << 3;
	// Room for many more flags
	
	#[inline] pub const fn empty() -> Self { Self(0) }
	#[inline] pub const fn new(flags: u32) -> Self { Self(flags) }
	#[inline] pub const fn inner(&self) -> u32 { self.0 }
	#[inline] pub const fn contains(&self, flag: u32) -> bool { (self.0 & flag) != 0 }
	#[inline] pub const fn with_flag(self, flag: u32) -> Self { Self(self.0 | flag) }
	#[inline] pub const fn without_flag(self, flag: u32) -> Self { Self(self.0 & !flag) }
	#[inline] pub const fn combine(self, other: Self) -> Self { Self(self.0 | other.0) }
}

#[derive(Debug, PartialEq)]
#[repr(C)] // Ensure predictable layout for better cache performance
pub struct ItemComp {
	pub id: ItemId,
	pub name: &'static str,
	pub max_stack: u32,
	pub flags: ItemFlags,
	// Optional data stored separately to avoid memory overhead when not needed
	pub data: Option<ItemExtendedData>,
}

impl ItemComp {
	pub const fn copy(&self) -> Self {
		Self {
			id: self.id,
			name: self.name,
			max_stack: self.max_stack,
			flags: self.flags,
			data: None,
		}
	}
	pub const fn new(id: u16, name: &'static str) -> Self {
		Self::new_i(ItemId(id), name)
	}
	pub const fn error() -> Self {
		Self::new_i(ItemId(0), "0")
	}
	pub const fn new_i(id: ItemId, name: &'static str) -> Self {
		Self {
			id,
			name,
			max_stack: 64u32,
			flags: ItemFlags::empty(),
			data: None,
		}
	}
	#[inline] pub const fn with_flag(mut self, flag: ItemFlags) -> Self {
		self.flags = self.flags.combine(flag);
		self
	}
	#[inline] pub const fn with_stack(mut self, max_stack: u32) -> Self {
		self.max_stack = max_stack;
		self
	}
	#[inline] pub fn with_data(self, data: ItemExtendedData) -> Self {
		// Note: This can't be const if Box::new isn't const
		// If you need it to be const, you'll need to adjust ItemExtendedData
		Self {
			data: Some(data),
			..self
		}
	}
	#[inline] pub const fn as_block(self) -> Self { 
		Self {
			flags: self.flags.with_flag(ItemFlags::IS_BLOCK),
			..self
		}
	}
	#[inline] pub const fn as_tool(self, tool_data: ToolData) -> Self {
		Self { 
			flags: self.flags.with_flag(ItemFlags::IS_TOOL),
			data: Some(ItemExtendedData::new().with_tool_data(tool_data)),
			..self 
		}
	}
	#[inline] pub const fn as_armor(self, armor_data: ArmorData) -> Self {
		Self { 
			flags: self.flags.with_flag(ItemFlags::IS_ARMOR),
			data: Some(ItemExtendedData::new().with_armor_data(armor_data)),
			..self 
		}
	}
	
	#[inline] pub const fn is_block(&self) -> bool { 
		self.flags.contains(ItemFlags::IS_BLOCK) 
	}
	#[inline] pub const fn has_durability(&self) -> bool { 
		self.flags.contains(ItemFlags::IS_TOOL) || self.flags.contains(ItemFlags::IS_ARMOR) 
	}
	#[inline] pub const fn is_tool(&self) -> bool { 
		self.flags.contains(ItemFlags::IS_TOOL) 
	}
	#[inline] pub fn is_weapon(&self) -> bool { 
		if let Some(data) = &self.data {
			data.get_damage().is_some()
		} else { false }
	}
	#[inline] pub const fn is_armor(&self) -> bool { 
		self.flags.contains(ItemFlags::IS_ARMOR) 
	}
}


/// All possible property types an item can have
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
	Durability(NonZeroU32),
	ToolData(ToolData),
	ArmorData(ArmorData),
	Hunger(i16),
	ArmorValue(i16),
	Damage(i16),
	Speed(i16),
}
impl PropertyValue {
	/// Returns the variant tag for this property (const-compatible)
	#[inline] pub const fn to_tag(&self) -> PropertyVariantTag {
		match self {
			Self::Durability(_) => PropertyVariantTag::Durability,
			Self::ToolData(_) => PropertyVariantTag::ToolData,
			Self::ArmorData(_) => PropertyVariantTag::ArmorData,
			Self::Hunger(_) => PropertyVariantTag::Hunger,
			Self::ArmorValue(_) => PropertyVariantTag::ArmorValue,
			Self::Damage(_) => PropertyVariantTag::Damage,
			Self::Speed(_) => PropertyVariantTag::Speed,
		}
	}
}
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[repr(u8)] // Ensure each variant has a u8 representation, so up to 255 type
pub enum PropertyVariantTag {
	Durability,
	ToolData,
	ArmorData,
	Hunger,
	ArmorValue,
	Damage,
	Speed,
}
// Manually implement PartialEq with const fn
impl PropertyVariantTag {
	#[inline] pub const fn eq(&self, other: &Self) -> bool {
		matches!(
			(self, other),
			(PropertyVariantTag::Durability, PropertyVariantTag::Durability)
			| (PropertyVariantTag::ToolData, PropertyVariantTag::ToolData)
			| (PropertyVariantTag::ArmorData, PropertyVariantTag::ArmorData)
			| (PropertyVariantTag::Hunger, PropertyVariantTag::Hunger)
			| (PropertyVariantTag::ArmorValue, PropertyVariantTag::ArmorValue)
			| (PropertyVariantTag::Damage, PropertyVariantTag::Damage)
			| (PropertyVariantTag::Speed, PropertyVariantTag::Speed)
		)
	}
	#[inline] pub const fn from_u8(value: u8) -> Option<Self> {
		unsafe { std::mem::transmute(value) }
	}
}
// Constants for configuration
const DEFAULT_MAX_PROPERTIES: usize = 4;

/// Ultra-compact storage using a fixed array of N properties
#[derive(Debug, Clone, PartialEq)]
pub struct ItemExtendedData<const N: usize = DEFAULT_MAX_PROPERTIES> where PropertyValue: PartialEq {
	pub data: [Option<PropertyValue>; N],
	pub len: u8, // Tracks how many properties are actually set
}
/*
// Default size (4)
let default_item = ItemExtendedData::new();

// Custom size
let small_item = ItemExtendedData::<2>::new();
let large_item = ItemExtendedData::<8>::new();
*/
impl<const N: usize> ItemExtendedData<N> {
	/// Creates a new empty ItemExtendedData with custom size N
	#[inline] pub const fn new() -> Self {
		Self {
			data: [const { None }; N],
			len: 0,
		}
	}

	// Property setters
	#[inline] pub const fn with_durability(self, value: NonZeroU32) -> Self {
		self.set_property(PropertyValue::Durability(value))
	}
	#[inline] pub const fn with_tool_data(self, value: ToolData) -> Self {
		self.set_property(PropertyValue::ToolData(value))
	}
	#[inline] pub const fn with_armor_data(self, value: ArmorData) -> Self {
		self.set_property(PropertyValue::ArmorData(value))
	}
	#[inline] pub const fn with_hunger(self, value: i16) -> Self {
		self.set_property(PropertyValue::Hunger(value))
	}
	#[inline] pub const fn with_armor(self, value: i16) -> Self {
		self.set_property(PropertyValue::ArmorValue(value))
	}
	#[inline] pub const fn with_damage(self, value: i16) -> Self {
		self.set_property(PropertyValue::Damage(value))
	}
	#[inline] pub const fn with_speed(self, value: i16) -> Self {
		self.set_property(PropertyValue::Speed(value))
	}

	/// Internal method to set or update a property
	#[inline] pub const fn set_property(mut self, new_value: PropertyValue) -> Self {
		// Check for existing property of same type
		if self.has_property(new_value.to_tag()) {
			return self;
		}

		// Add new property if we have space
		if (self.len()) < self.data.len() {
			self.data[self.len()] = Some(new_value);
			self.len += 1;
		}
		
		self
	}

	#[inline] pub const fn has_property(&self, expected_variant: PropertyVariantTag) -> bool {
		let mut i = 0;
		while i < self.len as usize {
			if let Some(prop) = &self.data[i] {
				// Compare the discriminant directly in const context
				if prop.to_tag().eq(&expected_variant) {
					return true;
				}
			}
			i += 1;
		}
		false
	}

	// Property getters
	#[inline] pub fn get_durability(&self) -> Option<NonZeroU32> {
		self.find_property(|v| match v {
			PropertyValue::Durability(d) => Some(*d),
			_ => None,
		})
	}
	#[inline] pub fn get_tool_data(&self) -> Option<&ToolData> {
		self.find_property_ref(|v| match v {
			PropertyValue::ToolData(t) => Some(t),
			_ => None,
		})
	}
	#[inline] pub fn get_armor_data(&self) -> Option<&ArmorData> {
		self.find_property_ref(|v| match v {
			PropertyValue::ArmorData(a) => Some(a),
			_ => None,
		})
	}
	#[inline] pub fn get_hunger(&self) -> Option<i16> {
		self.find_property(|v| match v {
			PropertyValue::Hunger(h) => Some(*h),
			_ => None,
		})
	}
	#[inline] pub fn get_armor_value(&self) -> Option<i16> {
		self.find_property(|v| match v {
			PropertyValue::ArmorValue(a) => Some(*a),
			_ => None,
		})
	}
	#[inline] pub fn get_damage(&self) -> Option<i16> {
		self.find_property(|v| match v {
			PropertyValue::Damage(d) => Some(*d),
			_ => None,
		})
	}
	#[inline] pub fn get_speed(&self) -> Option<i16> {
		self.find_property(|v| match v {
			PropertyValue::Speed(s) => Some(*s),
			_ => None,
		})
	}

	/// Helper method to find and transform a property
	#[inline] pub fn find_property<T, F>(&self, mut f: F) -> Option<T>
	where F: FnMut(&PropertyValue) -> Option<T>, {
		self.data.iter()
			.take(self.len())
			.filter_map(|slot| slot.as_ref())
			.find_map(|prop| f(prop))
	}

	/// Helper method to find and return a reference to a property
	#[inline] pub fn find_property_ref<T, F>(&self, mut f: F) -> Option<&T>
	where F: FnMut(&PropertyValue) -> Option<&T>, {
		self.data.iter()
			.take(self.len())
			.filter_map(|slot| slot.as_ref())
			.find_map(|prop| f(prop))
	}

	/// Returns the current number of properties
	#[inline] pub const fn len(&self) -> usize {
		self.len as usize
	}

	/// Returns the maximum capacity of properties
	#[inline] pub const fn capacity(&self) -> usize {
		N
	}
}








// Type aliases for convenience
pub type ToolTypeSet = EquipmentTypeSet<ToolType, u8>;
pub type ArmorTypeSet = EquipmentTypeSet<ArmorType, u16>;

pub type ToolData = EquipmentData<ToolType, ToolSet>;
pub type ArmorData = EquipmentData<ArmorType, ArmorSet>;

pub type ToolSet = EquipmentSetStruct<ToolType, u8, u64>;
pub type ArmorSet = EquipmentSetStruct<ArmorType, u16, u128>;

pub trait EquipmentType: Copy + Clone {
	const MAX_VARIANTS: u8;
	const TO_U8: fn(Self) -> u8;
	
	// Add a safe conversion from u8
	fn from_u8(value: u8) -> Option<Self>;
}
impl EquipmentType for ToolType {
	const MAX_VARIANTS: u8 = 8;
	const TO_U8: fn(Self) -> u8 = |x| x as u8;
	
	#[inline] fn from_u8(value: u8) -> Option<Self> {
		unsafe { std::mem::transmute(value) }
	}
}
impl EquipmentType for ArmorType {
	const MAX_VARIANTS: u8 = 16;
	const TO_U8: fn(Self) -> u8 = |x| x as u8;
	
	#[inline] fn from_u8(value: u8) -> Option<Self> {
		unsafe { std::mem::transmute(value) }
	}
}

// Generic equipment data struct
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EquipmentTypeSet<T: EquipmentType, S> {
	pub slots: S,
	pub _phantom: PhantomData<T>,
}

// Trait to handle bit operations for different storage types
pub trait BitStorage: Copy + Clone + PartialEq + Eq {
	const ZERO: Self;
	fn set_bit(&mut self, bit: u8);
	fn clear_bit(&mut self, bit: u8);
	fn get_bit(&self, bit: u8) -> bool;
}
impl BitStorage for u8 {
	const ZERO: Self = 0;
	#[inline] fn set_bit(&mut self, bit: u8) { *self |= 1 << bit; }
	#[inline] fn clear_bit(&mut self, bit: u8) { *self &= !(1 << bit); }
	#[inline] fn get_bit(&self, bit: u8) -> bool { (*self & (1 << bit)) != 0 }
}
impl BitStorage for u16 {
	const ZERO: Self = 0;
	#[inline] fn set_bit(&mut self, bit: u8) { *self |= 1 << bit; }
	#[inline] fn clear_bit(&mut self, bit: u8) { *self &= !(1 << bit); }
	#[inline] fn get_bit(&self, bit: u8) -> bool { (*self & (1 << bit)) != 0 }
}

impl<T: EquipmentType, S: BitStorage> EquipmentTypeSet<T, S> {
	#[inline] pub const fn new() -> Self { 
		Self { 
			slots: S::ZERO,
			_phantom: PhantomData,
		}
	}
	
	#[inline] pub fn on_u(&mut self, slot: u8) { 
		self.slots.set_bit(slot);
	}
	#[inline] pub fn off_u(&mut self, slot: u8) { 
		self.slots.clear_bit(slot);
	}
	#[inline] pub fn is_u(&self, slot: u8) -> bool { 
		self.slots.get_bit(slot)
	}
	#[inline] pub fn is_empty(&self) -> bool { 
		self.slots == S::ZERO
	}
	#[inline] pub fn on(&mut self, slot: T) { 
		self.slots.set_bit(T::TO_U8(slot));
	}
	#[inline] pub fn off(&mut self, slot: T) { 
		self.slots.clear_bit(T::TO_U8(slot));
	}
	#[inline] pub fn is(&self, slot: T) -> bool { 
		self.slots.get_bit(T::TO_U8(slot))
	}
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

// Renamed to avoid conflict with trait
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EquipmentSetStruct<T: EquipmentType, S: BitStorage, TierStorage> {
	// Bits for tracking which types are present
	pub types: EquipmentTypeSet<T, S>,
	// Packed tiers for each possible type
	pub tiers: TierStorage,
}

impl<T: EquipmentType, S: BitStorage, TS: TierStorage> EquipmentSetStruct<T, S, TS> {
	#[inline] pub const fn new() -> Self {
		EquipmentSetStruct {
			types: EquipmentTypeSet::new(),
			tiers: TS::ZERO,
		}
	}
}

// Trait for tier storage operations
pub trait TierStorage: Copy + Clone + PartialEq + Eq {
	const ZERO: Self;
	fn set_tier(&mut self, index: u8, tier: u8);
	fn get_tier(&self, index: u8) -> u8;
	fn max_types() -> u8;
}
impl TierStorage for u64 {
	const ZERO: Self = 0;
	#[inline] fn set_tier(&mut self, index: u8, tier: u8) {
		let shift = index * 8;
		*self &= !(0xFF << shift); // Clear the existing tier
		*self |= (tier as u64) << shift; // Set the new tier
	}
	#[inline] fn get_tier(&self, index: u8) -> u8 { ((self >> (index * 8)) & 0xFF) as u8 }
	#[inline] fn max_types() -> u8 { 8 }
}
impl TierStorage for u128 {
	const ZERO: Self = 0;
	#[inline] fn set_tier(&mut self, index: u8, tier: u8) {
		let shift = index * 8;
		*self &= !(0xFF << shift); // Clear the existing tier
		*self |= (tier as u128) << shift; // Set the new tier
	}
	#[inline] fn get_tier(&self, index: u8) -> u8 { ((self >> (index * 8)) & 0xFF) as u8 }
	#[inline] fn max_types() -> u8 { 16 }
}

impl<T: EquipmentType, S: BitStorage, TS: TierStorage> EquipmentSetStruct<T, S, TS> {
	#[inline] 
	pub fn add_equipment(&mut self, equip_type: T, tier: MaterialLevel) {
		if self.has_equipment(equip_type) {
			panic!("Equipment type already exists");
		}
		let index = T::TO_U8(equip_type);
		self.types.on_u(index);
		self.set_tier(index, tier as u8);
	}
	
	#[inline] pub fn remove_equipment(&mut self, equip_type: T) {
		if !self.has_equipment(equip_type) {
			return;
		}
		let index = T::TO_U8(equip_type);
		self.types.off_u(index);
		self.set_tier(index, 0);
	}
	
	#[inline] pub fn has_equipment(&self, equip_type: T) -> bool {
		self.types.is(equip_type)
	}

	#[inline] 
	pub fn get_tier(&self, equip_type: T) -> Option<MaterialLevel> {
		if !self.has_equipment(equip_type) { return None; };
		
		let index = T::TO_U8(equip_type);
		let value = TS::get_tier(&self.tiers, index);
		MaterialLevel::from_u8(value)
	}

	#[inline] 
	pub fn iter(&self) -> impl Iterator<Item = (T, MaterialLevel)> + '_ {
		(0..TS::max_types()).filter_map(move |i| {
			if !self.types.is_u(i) { return None; }

			let value = TS::get_tier(&self.tiers, i);
			let (Some(equip_type), Some(tier)) = (T::from_u8(i), MaterialLevel::from_u8(value)) 
				else { return None; };
			
			Some((equip_type, tier))
		})
	}

	#[inline] 
	fn set_tier(&mut self, index: u8, tier: u8) {
		TS::set_tier(&mut self.tiers, index, tier);
	}
}

// Trait for equipment sets (renamed to avoid conflict)
pub trait EquipmentSet<T: EquipmentType> {
	fn get_tier(&self, equip_type: T) -> Option<MaterialLevel>;
}

// Implement EquipmentSet trait for the generic version
impl<T: EquipmentType, S: BitStorage, TS: TierStorage> EquipmentSet<T> for EquipmentSetStruct<T, S, TS> {
	fn get_tier(&self, equip_type: T) -> Option<MaterialLevel> {
		self.get_tier(equip_type)
	}
}

// Generic equipment data enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EquipmentData<T: EquipmentType, S> {
	None,
	Single { equip_type: T, tier: MaterialLevel },
	Multiple(S),
}

impl<T: EquipmentType + PartialEq, S: EquipmentSet<T>> EquipmentData<T, S> {
	pub const fn none() -> Self {
		Self::None
	}
	
	pub const fn single(equip_type: T, tier: MaterialLevel) -> Self {
		Self::Single { equip_type, tier }
	}
	
	#[inline]
	pub const fn is_equipment(&self) -> bool {
		!matches!(self, Self::None)
	}
	
	pub fn get_tier(&self, equip_type: T) -> Option<MaterialLevel> {
		match self {
			Self::None => None,
			Self::Single { equip_type: t, tier } if *t == equip_type => Some(*tier),
			Self::Single { .. } => None,
			Self::Multiple(set) => set.get_tier(equip_type),
		}
	}
	
	#[inline]
	pub const fn as_single(&self) -> Option<(T, MaterialLevel)> {
		match self {
			Self::Single { equip_type, tier } => Some((*equip_type, *tier)),
			_ => None,
		}
	}
}
