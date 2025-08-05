use crate::item::items::ItemId;
use crate::item::material::MaterialLevel;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{self, Visitor};
use crate::item::item_lut::{ItemFlags, ItemComp, PropertyVariantTag, ToolType, ArmorType, PropertyValue, ItemExtendedData, TierStorage, EquipmentTypeSet, EquipmentData, EquipmentSetStruct, EquipmentType, BitStorage};
use std::fmt;


// ItemFlags serialization - as u32
impl Serialize for ItemFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for ItemFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Ok(ItemFlags(u32::deserialize(deserializer)?))
    }
}

// PropertyVariantTag serialization - as u8
impl Serialize for PropertyVariantTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for PropertyVariantTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = u8::deserialize(deserializer)?;
        PropertyVariantTag::from_u8(value)
            .ok_or_else(|| de::Error::custom(format!("Invalid PropertyVariantTag: {}", value)))
    }
}

// ToolType serialization - as u8
impl Serialize for ToolType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for ToolType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = u8::deserialize(deserializer)?;
        ToolType::from_u8(value)
            .ok_or_else(|| de::Error::custom(format!("Invalid ToolType: {}", value)))
    }
}

// ArmorType serialization - as u8
impl Serialize for ArmorType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for ArmorType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = u8::deserialize(deserializer)?;
        ArmorType::from_u8(value)
            .ok_or_else(|| de::Error::custom(format!("Invalid ArmorType: {}", value)))
    }
}

// PropertyValue compact serialization - [tag, value]
impl Serialize for PropertyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeTuple;
        
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&(self.to_tag() as u8))?;
        
        match self {
            PropertyValue::Durability(d) => tuple.serialize_element(&d.get())?,
            PropertyValue::ToolData(t) => tuple.serialize_element(t)?,
            PropertyValue::ArmorData(a) => tuple.serialize_element(a)?,
            PropertyValue::Hunger(h) => tuple.serialize_element(h)?,
            PropertyValue::ArmorValue(a) => tuple.serialize_element(a)?,
            PropertyValue::Damage(d) => tuple.serialize_element(d)?,
            PropertyValue::Speed(s) => tuple.serialize_element(s)?,
        }
        
        tuple.end()
    }
}

struct PropertyValueVisitor;

impl<'de> Visitor<'de> for PropertyValueVisitor {
    type Value = PropertyValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple [tag, value]")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: de::SeqAccess<'de> {
        let tag: u8 = seq.next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        
        let variant = PropertyVariantTag::from_u8(tag)
            .ok_or_else(|| de::Error::custom(format!("Invalid PropertyVariantTag: {}", tag)))?;
        
        match variant {
            PropertyVariantTag::Durability => {
                let val: u32 = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let nz = std::num::NonZeroU32::new(val)
                    .ok_or_else(|| de::Error::custom("Durability cannot be zero"))?;
                Ok(PropertyValue::Durability(nz))
            },
            PropertyVariantTag::ToolData => {
                let val = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(PropertyValue::ToolData(val))
            },
            PropertyVariantTag::ArmorData => {
                let val = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(PropertyValue::ArmorData(val))
            },
            PropertyVariantTag::Hunger => {
                let val = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(PropertyValue::Hunger(val))
            },
            PropertyVariantTag::ArmorValue => {
                let val = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(PropertyValue::ArmorValue(val))
            },
            PropertyVariantTag::Damage => {
                let val = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(PropertyValue::Damage(val))
            },
            PropertyVariantTag::Speed => {
                let val = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(PropertyValue::Speed(val))
            },
        }
    }
}

impl<'de> Deserialize<'de> for PropertyValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_tuple(2, PropertyValueVisitor)
    }
}

// ItemExtendedData compact serialization - [data_array, len]
impl<const N: usize> Serialize for ItemExtendedData<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeTuple;
        
        let mut tuple = serializer.serialize_tuple(2)?;
        
        // Only serialize the used portion of the array
        let used_data: Vec<&PropertyValue> = self.data.iter()
            .take(self.len as usize)
            .filter_map(|opt| opt.as_ref())
            .collect();
        
        tuple.serialize_element(&used_data)?;
        tuple.serialize_element(&self.len)?;
        tuple.end()
    }
}

struct ItemExtendedDataVisitor<const N: usize>;

impl<'de, const N: usize> Visitor<'de> for ItemExtendedDataVisitor<N> {
    type Value = ItemExtendedData<N>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple [data_array, len]")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: de::SeqAccess<'de> {
        let data_vec: Vec<PropertyValue> = seq.next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let len: u8 = seq.next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        
        if data_vec.len() > N {
            return Err(de::Error::custom(format!("Too many properties: {} > {}", data_vec.len(), N)));
        }
        
        let mut data = [const { None }; N];
        for (i, prop) in data_vec.into_iter().enumerate() {
            data[i] = Some(prop);
        }
        
        Ok(ItemExtendedData { data, len })
    }
}

impl<'de, const N: usize> Deserialize<'de> for ItemExtendedData<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_tuple(2, ItemExtendedDataVisitor::<N>)
    }
}

// ItemComp compact serialization
impl Serialize for ItemComp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeStruct;
        
        let mut state = serializer.serialize_struct("ItemComp", 5)?;
        state.serialize_field("id", &self.id.0)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("max_stack", &self.max_stack)?;
        state.serialize_field("flags", &self.flags)?;
        state.serialize_field("data", &self.data)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ItemComp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field { Id, Name, MaxStack, Flags, Data }

        struct ItemCompVisitor;

        impl<'de> Visitor<'de> for ItemCompVisitor {
            type Value = ItemComp;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ItemComp")
            }

            fn visit_map<V>(self, mut map: V) -> Result<ItemComp, V::Error>
            where V: de::MapAccess<'de> {
                let mut id = None;
                let mut name = None;
                let mut max_stack = None;
                let mut flags = None;
                let mut data = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            let id_val: u16 = map.next_value()?;
                            id = Some(ItemId(id_val));
                        }
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::MaxStack => {
                            if max_stack.is_some() {
                                return Err(de::Error::duplicate_field("max_stack"));
                            }
                            max_stack = Some(map.next_value()?);
                        }
                        Field::Flags => {
                            if flags.is_some() {
                                return Err(de::Error::duplicate_field("flags"));
                            }
                            flags = Some(map.next_value()?);
                        }
                        Field::Data => {
                            if data.is_some() {
                                return Err(de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let name: &str = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let max_stack = max_stack.ok_or_else(|| de::Error::missing_field("max_stack"))?;
                let flags = flags.ok_or_else(|| de::Error::missing_field("flags"))?;
                let data = data.ok_or_else(|| de::Error::missing_field("data"))?;

                let static_name: &'static str = Box::leak(name.to_string().into_boxed_str());
                Ok(ItemComp { id, name: static_name, max_stack, flags, data })
            }
        }

        const FIELDS: &'static [&'static str] = &["id", "name", "max_stack", "flags", "data"];
        deserializer.deserialize_struct("ItemComp", FIELDS, ItemCompVisitor)
    }
}

// EquipmentTypeSet compact serialization - just the bits
impl<T: EquipmentType, S: BitStorage + Serialize> Serialize for EquipmentTypeSet<T, S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where Ser: Serializer {
        self.slots.serialize(serializer)
    }
}

impl<'de, T: EquipmentType, S: BitStorage + Deserialize<'de>> Deserialize<'de> for EquipmentTypeSet<T, S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let slots = S::deserialize(deserializer)?;
        Ok(Self {
            slots,
            _phantom: std::marker::PhantomData,
        })
    }
}

// EquipmentSetStruct compact serialization - [types, tiers]
impl<T: EquipmentType, S: BitStorage + Serialize, TS: TierStorage + Serialize> Serialize for EquipmentSetStruct<T, S, TS> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where Ser: Serializer {
        use serde::ser::SerializeTuple;
        
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.types)?;
        tuple.serialize_element(&self.tiers)?;
        tuple.end()
    }
}

struct EquipmentSetStructVisitor<T, S, TS> {
    _phantom: std::marker::PhantomData<(T, S, TS)>,
}

impl<'de, T: EquipmentType, S: BitStorage + Deserialize<'de>, TS: TierStorage + Deserialize<'de>> Visitor<'de> for EquipmentSetStructVisitor<T, S, TS> {
    type Value = EquipmentSetStruct<T, S, TS>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple [types, tiers]")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: de::SeqAccess<'de> {
        let types = seq.next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let tiers = seq.next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        
        Ok(EquipmentSetStruct { types, tiers })
    }
}

impl<'de, T: EquipmentType, S: BitStorage + Deserialize<'de>, TS: TierStorage + Deserialize<'de>> Deserialize<'de> for EquipmentSetStruct<T, S, TS> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_tuple(2, EquipmentSetStructVisitor {
            _phantom: std::marker::PhantomData,
        })
    }
}

// EquipmentData compact serialization
impl<T: EquipmentType + Serialize, S: Serialize> Serialize for EquipmentData<T, S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where Ser: Serializer {
        use serde::ser::SerializeTuple;
        
        match self {
            Self::None => {
                let mut tuple = serializer.serialize_tuple(1)?;
                tuple.serialize_element(&0u8)?;
                tuple.end()
            },
            Self::Single { equip_type, tier } => {
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element(&1u8)?;
                tuple.serialize_element(equip_type)?;
                tuple.serialize_element(&(*tier as u8))?;
                tuple.end()
            },
            Self::Multiple(set) => {
                let mut tuple = serializer.serialize_tuple(2)?;
                tuple.serialize_element(&2u8)?;
                tuple.serialize_element(set)?;
                tuple.end()
            },
        }
    }
}

struct EquipmentDataVisitor<T, S> {
    _phantom: std::marker::PhantomData<(T, S)>,
}

impl<'de, T: EquipmentType + Deserialize<'de>, S: Deserialize<'de>> Visitor<'de> for EquipmentDataVisitor<T, S> {
    type Value = EquipmentData<T, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple representing EquipmentData")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: de::SeqAccess<'de> {
        let tag: u8 = seq.next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        
        match tag {
            0 => Ok(EquipmentData::None),
            1 => {
                let equip_type = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let tier_val: u8 = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let tier = MaterialLevel::from_u8(tier_val)
                    .ok_or_else(|| de::Error::custom(format!("Invalid MaterialLevel: {}", tier_val)))?;
                Ok(EquipmentData::Single { equip_type, tier })
            },
            2 => {
                let set = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(EquipmentData::Multiple(set))
            },
            _ => Err(de::Error::custom(format!("Invalid EquipmentData tag: {}", tag))),
        }
    }
}

impl<'de, T: EquipmentType + Deserialize<'de>, S: Deserialize<'de>> Deserialize<'de> for EquipmentData<T, S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_tuple(3, EquipmentDataVisitor {
            _phantom: std::marker::PhantomData,
        })
    }
}
