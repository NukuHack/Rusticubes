
item and related things also the inventory stuff


/// Item + Inventory and related stuffs
pub mod item {
	// Item and Itemstack body, inventory basics
	pub mod items;
	/// Item related things what will not change at runtime
	pub mod item_lut;
	/// Binary serialization and de-serialization for items
	pub mod item_binary;
	/// Json de-serialization for items
	pub mod item_json;
	/// Basic corner-stone of the item system
	pub mod material;
	/// Main inventory implementation, and Item grid impl.
	pub mod inventory;
	/// the rendering part of the inventory and the items
	pub mod ui_inventory;
	/// the recipes, for now only for crafting
	pub mod recipes;
}

