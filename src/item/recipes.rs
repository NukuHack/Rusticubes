use std::collections::HashSet;
use ahash::RandomState;
use std::sync::{OnceLock, RwLock, RwLockReadGuard};
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

impl Hash for Recipe {
	fn hash<H: Hasher>(&self, state: &mut H) {
		// Only hash the input since outputs aren't used for lookups
		self.input.hash(state);
	}
}

impl Recipe {
	#[inline]
	pub fn new<T: Into<CraftingInput>, K: Into<CraftingResult>>(input: T, output: K) -> Self {
		Self {
			input: input.into(),
			output: output.into(),
		}
	}

	#[inline]
	pub fn input(&self) -> &CraftingInput {
		&self.input
	}

	#[inline]
	pub fn output(&self) -> &CraftingResult {
		&self.output
	}
}

// Global recipe registry
static RECIPE_REGISTRY: OnceLock<RwLock<HashSet<Recipe, RandomState>>> = OnceLock::new();

/// Initializes the recipe registry with default recipes
pub fn init_recipe_registry() {
	RECIPE_REGISTRY.get_or_init(|| RwLock::new(HashSet::with_hasher(RandomState::new())));
}

/// Clears all recipes from the registry
pub fn clear_recipes() {
	if let Some(registry) = RECIPE_REGISTRY.get() {
		registry.write().expect("Recipe registry poisoned").clear();
	}
}

/// Gets a read-only reference to the recipe registry
pub fn get_recipes() -> RwLockReadGuard<'static, HashSet<Recipe, RandomState>> {
	RECIPE_REGISTRY.get()
		.expect("Recipe registry not initialized")
		.read().expect("Recipe registry poisoned")
}

/// Initializes the recipe lookup table with default recipes
pub fn init_recipe_lut() {
	init_recipe_registry();

	let mut recipes = RECIPE_REGISTRY.get()
		.expect("Recipe registry not initialized")
		.write().expect("Recipe registry poisoned");

	recipes.extend([
		Recipe::new(0, 0),
		Recipe::new(1, 1),
		Recipe::new(2, 2),
		Recipe::new(13, 13), // Add this line for brick_grey
		// other recipes
	]);
}

/// Looks up a recipe by input only
pub fn lookup_recipe(input: &CraftingInput) -> Option<CraftingResult> {
	let registry = get_recipes();
	// Create a temporary recipe with dummy output for lookup
	let lookup_recipe = Recipe {
		input: input.clone(),
		output: 0.into(), // dummy value
	};
	
	registry.get(&lookup_recipe).map(|r| r.output().clone())
}



use crate::item::inventory::ItemContainer;

impl ItemContainer {
	/// Converts the container's contents into a CraftingInput for recipe lookup
	pub fn to_crafting_input(&self) -> CraftingInput {
		let mut requirements = Vec::new();
		
		// Calculate center offsets for positioning
		let y_offset = -(self.rows() as i8 / 2);
		let x_offset = -(self.cols() as i8 / 2);
		
		// Iterate through all slots in row-major order
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
			1 => CraftingInput::Single(requirements[0].item_id),
			_ => CraftingInput::Multiple(requirements),
		}
	}
	
	/// Looks up a recipe matching this container's contents
	pub fn find_recipe(&self) -> Option<CraftingResult> {
		let input = self.to_crafting_input();
		println!("Crafting input: {:?}", input); // Add this line
		lookup_recipe(&input)
	}
}

impl CraftingResult {
	// have to impl converting back to ItemContainer or atleast a Vec<> of ItemStack
}
