
/// A string that can be either static or dynamic.
///
/// This type is useful when you want to avoid allocations for static strings
/// but still need the ability to mutate the string when necessary.
#[derive(Debug, Clone)]
pub enum MutStr {
	/// A static string reference
	Static(&'static str),
	/// A dynamically allocated string
	Dynamic(String),
}

impl MutStr {
	// Construction
	pub const fn from_str(s: &'static str) -> Self {
		Self::Static(s)
	}
	pub const fn default() -> Self {
		Self::Static("")
	}
	
	// Conversion
	pub fn into_string(self) -> String {
		match self {
			Self::Static(s) => s.to_string(),
			Self::Dynamic(s) => s,
		}
	}

	pub const fn to_op(s: Option<&'static str>) -> Option<Self> {
		match s {
			Some(s) => Some(Self::Static(s)),
			None => Some(Self::default()),
		}
	}
	pub const fn o(&self) -> Option<&Self> {
		Some(self)
	}

	// Basic operations
	pub fn to_str(&self) -> &str {
		match self {
			Self::Static(s) => s,
			Self::Dynamic(s) => s.as_str(),
		}
	}

	pub fn get_mut(&mut self) -> &mut String {
		self.ensure_mutable();
		match self {
			Self::Dynamic(s) => s,
			_ => unreachable!(),
		}
	}
	
	pub fn len(&self) -> usize {
		match self {
			Self::Static(s) => s.len(),
			Self::Dynamic(s) => s.len(),
		}
	}
	
	pub fn is_empty(&self) -> bool {
		match self {
			Self::Static(s) => s.is_empty(),
			Self::Dynamic(s) => s.is_empty(),
		}
	}
	
	// Mutation operations
	pub fn push_str(&mut self, s: &str) {
		self.ensure_mutable();
		if let Self::Dynamic(string) = self {
			string.push_str(s);
		}
	}
	
	pub fn push(&mut self, c: char) {
		self.ensure_mutable();
		if let Self::Dynamic(string) = self {
			string.push(c);
		}
	}
	
	pub fn pop(&mut self) -> Option<char> {
		match self {
			Self::Static(s) if !s.is_empty() => {
				let mut chars = s.chars();
				let last_char = chars.next_back();
				let new_str = chars.as_str();
				// Convert to owned string minus the last character
				*self = Self::Dynamic(new_str.to_string());
				last_char
			}
			Self::Static(_) => None,
			Self::Dynamic(s) => s.pop(),
		}
	}
	
	pub fn clear(&mut self) {
		*self = Self::Dynamic(String::new());
	}
	
	pub fn truncate(&mut self, new_len: usize) {
		match self {
			Self::Static(s) if new_len < s.len() => {
				*self = Self::Dynamic(s[..new_len].to_string());
			}
			Self::Dynamic(s) => s.truncate(new_len),
			_ => {}
		}
	}
	
	pub fn remove(&mut self, idx: usize) -> char {
		self.ensure_mutable();
		if let Self::Dynamic(s) = self {
			s.remove(idx)
		} else { // ensure_mutable converted to Dynamic
			unreachable!()
		}
	}
	
	pub fn retain<F>(&mut self, f: F)
	where
		F: FnMut(char) -> bool,
	{
		self.ensure_mutable();
		if let Self::Dynamic(s) = self {
			s.retain(f);
		}
	}
	
	// Helper to convert to mutable version when needed
	fn ensure_mutable(&mut self) {
		if let Self::Static(s) = *self {
			*self = Self::Dynamic(s.to_string());
		}
	}


	pub fn starts_with(&self, pat: &str) -> bool {
		self.to_str().starts_with(pat)
	}
	
	pub fn ends_with(&self, pat: &str) -> bool {
		self.to_str().ends_with(pat)
	}
	
	pub fn contains(&self, pat: &str) -> bool {
		self.to_str().contains(pat)
	}
	
	pub fn split<'a>(&'a self, pat: &'a str) -> std::str::Split<'a, &'a str> {
		self.to_str().split(pat)
	}
}


// Common trait implementations
impl From<&'static str> for MutStr {
	fn from(s: &'static str) -> Self {
		Self::Static(s)
	}
}
impl From<String> for MutStr {
	fn from(s: String) -> Self {
		Self::Dynamic(s)
	}
}
impl From<&String> for MutStr {
	fn from(s: &String) -> Self {
		Self::Dynamic(s.clone())
	}
}
impl From<&mut String> for MutStr {
	fn from(s: &mut String) -> Self {
		Self::Dynamic(std::mem::take(s))
	}
}

impl std::fmt::Display for MutStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_str())
	}
}

use std::ops::Deref;
use std::option::Option;
use std::borrow::Borrow;

impl Deref for MutStr {
	type Target = str;
	
	fn deref(&self) -> &str {
		self.to_str()
	}
}
impl AsRef<str> for MutStr {
	fn as_ref(&self) -> &str {
		self.to_str()
	}
}
impl AsRef<[u8]> for MutStr {
	fn as_ref(&self) -> &[u8] {
		self.to_str().as_bytes()
	}
}
impl Borrow<str> for MutStr {
	fn borrow(&self) -> &str {
		self.to_str()
	}
}




// addition so i don't have to use format!() all the time ...

use std::ops::Add;

impl Add<&str> for MutStr {
	type Output = Self;

	fn add(mut self, rhs: &str) -> Self::Output {
		self.push_str(rhs);
		self
	}
}

impl Add<String> for MutStr {
	type Output = Self;

	fn add(mut self, rhs: String) -> Self::Output {
		self.push_str(&rhs);
		self
	}
}

impl Add<MutStr> for MutStr {
	type Output = Self;

	fn add(mut self, rhs: Self) -> Self::Output {
		self.push_str(rhs.to_str());
		self
	}
}

impl Add<&MutStr> for &MutStr {
	type Output = MutStr;
	
	fn add(self, rhs: &MutStr) -> Self::Output {
		let mut result = self.clone();
		result.push_str(rhs.to_str());
		result
	}
}
