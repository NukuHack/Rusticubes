
use crate::item::items::ItemStack;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard};
use ahash::RandomState;
use std::hash::{Hash, Hasher};

/// Position relative to the center item in a crafting grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(packed)]
pub struct GridPosition { pub x: i8, pub y: i8 }

impl GridPosition {
	#[inline] pub const fn new(x: i8, y: i8) -> Self { Self { x, y } }
}

/// Item requirement at a specific grid position
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRequirement {
	pub item_id: usize,
	pub position: GridPosition,
}

impl Hash for ItemRequirement {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.item_id.hash(state);
		self.position.hash(state);
	}
}

impl ItemRequirement {
	#[inline]
	pub const fn new(item_id: usize, x: i8, y: i8) -> Self {
		Self {
			item_id,
			position: GridPosition::new(x, y),
		}
	}
}

/// Requirements of a crafting operation (used for lookups)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftingInput {
	Single(usize),
	Multiple(Vec<ItemRequirement>),
}

impl Hash for CraftingInput {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			CraftingInput::Single(id) => {
				0u8.hash(state); // variant discriminator
				id.hash(state);
			}
			CraftingInput::Multiple(items) => {
				1u8.hash(state); // variant discriminator
				for item in items {
					item.hash(state);
				}
			}
		}
	}
}

impl From<Vec<Vec<usize>>> for CraftingInput {
	fn from(items: Vec<Vec<usize>>) -> Self {
		let mut requirements = Vec::new();
		
		if !items.is_empty() {
			let height = items.len();
			let width = items[0].len();
			let y_offset = -(height as i8 / 2);
			let x_offset = -(width as i8 / 2);
			
			for (y, row) in items.into_iter().enumerate() {
				for (x, item_id) in row.into_iter().enumerate() {
					if item_id != 0 {
						requirements.push(ItemRequirement::new(
							item_id,
							x as i8 + x_offset,
							y as i8 + y_offset,
						));
					}
				}
			}
		}
		
		match requirements.len() {
			1 => Self::Single(requirements[0].item_id),
			_ => Self::Multiple(requirements),
		}
	}
}

impl From<Vec<ItemRequirement>> for CraftingInput {
	#[inline]
	fn from(items: Vec<ItemRequirement>) -> Self {
		Self::Multiple(items)
	}
}

impl From<&[ItemRequirement]> for CraftingInput {
	#[inline]
	fn from(items: &[ItemRequirement]) -> Self {
		if items.len() == 1 {
			Self::Single(items[0].item_id)
		} else {
			Self::Multiple(items.into())
		}
	}
}

impl From<usize> for CraftingInput {
	#[inline]
	fn from(item_id: usize) -> Self {
		Self::Single(item_id)
	}
}

/// Result of a crafting operation (not used for lookups)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftingResult {
	Single(usize),
	Multiple(Vec<usize>),
}

impl From<usize> for CraftingResult {
	#[inline]
	fn from(item_id: usize) -> Self {
		Self::Single(item_id)
	}
}

impl From<Vec<usize>> for CraftingResult {
	#[inline]
	fn from(item_ids: Vec<usize>) -> Self {
		Self::Multiple(item_ids)
	}
}

impl From<&[usize]> for CraftingResult {
	#[inline]
	fn from(item_ids: &[usize]) -> Self {
		if item_ids.len() == 1 {
			Self::Single(item_ids[0])
		} else {
			Self::Multiple(item_ids.into())
		}
	}
}

/// Recipe data defining input requirements and output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
	input: CraftingInput,
	output: CraftingResult,
}

impl Recipe {
	#[inline]
	pub fn new<T: Into<CraftingInput>, K: Into<CraftingResult>>(input: T, output: K) -> Self {
		Self {
			input: input.into(),
			output: output.into(),
		}
	}

	#[inline] pub fn split(self) -> (CraftingInput, CraftingResult) { (self.input, self.output) }
	#[inline] pub fn input(&self) -> &CraftingInput { &self.input }
	#[inline] pub fn output(&self) -> &CraftingResult { &self.output }
}

// Replace the HashSet with HashMap
static RECIPE_REGISTRY: OnceLock<RwLock<HashMap<CraftingInput, CraftingResult, RandomState>>> = OnceLock::new();

pub fn init_recipe_registry() {
	RECIPE_REGISTRY.get_or_init(|| RwLock::new(HashMap::with_hasher(RandomState::new())));
}

/// Clears all recipes from the registry
pub fn clear_recipes() {
	if let Some(registry) = RECIPE_REGISTRY.get() {
		registry.write().expect("Recipe registry poisoned").clear();
	}
}

/// Gets a read-only reference to the recipe registry
pub fn get_recipes() -> RwLockReadGuard<'static, HashMap<CraftingInput, CraftingResult, RandomState>> {
	RECIPE_REGISTRY.get()
		.expect("Recipe registry not initialized")
		.read().expect("Recipe registry poisoned")
}

pub fn init_recipe_lut() {
	init_recipe_registry();
	
	let mut recipes = RECIPE_REGISTRY.get()
		.expect("Recipe registry not initialized")
		.write().expect("Recipe registry poisoned");

	let num = ItemStack::from_str("brick_grey").resource_index();
	let crafting_table = ItemStack::from_str("crafting").resource_index();
	let o_shape = vec![vec![num,num,num],vec![num,0,num],vec![num,num,num]];
	
	recipes.extend([
		Recipe::new(num,num).split(),
		Recipe::new(o_shape,crafting_table).split(),
		// other recipes
	]);
}

pub fn lookup_recipe(input: &CraftingInput) -> Option<CraftingResult> {
	let registry = get_recipes();
	registry.get(input).cloned()
}

pub fn print_all_recipes() {
	let registry = get_recipes();
	println!("Registered recipes:");
	for recipe in registry.iter() {
		println!("- {:?}", recipe);
	}
}

use crate::item::inventory::ItemContainer;

impl ItemContainer {
	pub fn to_crafting_input(&self) -> Option<CraftingInput> {  // Changed to return Option
		let mut requirements = Vec::new();
		
		// Calculate center offsets
		let y_offset = -(self.rows() as i8 / 2);
		let x_offset = -(self.cols() as i8 / 2);
		
		for row in 0..self.rows() {
			for col in 0..self.cols() {
				if let Some(item_stack) = self.get_at(row, col) {
					requirements.push(ItemRequirement::new(
						item_stack.resource_index(),
						col as i8 + x_offset,
						row as i8 + y_offset,
					));
				}
			}
		}
		
		match requirements.len() {
			0 => None,
			1 => Some(CraftingInput::Single(requirements[0].item_id)),
			_ => Some(CraftingInput::Multiple(requirements)),
		}
	}

	pub fn find_recipe(&self) -> Option<CraftingResult> {
		self.to_crafting_input().and_then(|input| {
			//println!("Valid crafting input: {:?}", input);  // Debug log
			lookup_recipe(&input)
		})
	}
}

impl CraftingResult {
	// have to impl converting back to ItemContainer or atleast a Vec<> of ItemStack
}
