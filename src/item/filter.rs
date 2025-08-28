

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemFilter {
	i : u8
}

impl ItemFilter {
	pub fn default() -> Self {
		Self{ i:0 }
	}
}
