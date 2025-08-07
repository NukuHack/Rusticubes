impl ItemId {
	pub const fn from_str(string: &'static str) -> Self {
		const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
			if a.len() != b.len() {
				return false;
			}
			let mut i = 0;
			while i < a.len() {
				if a[i] != b[i] {
					return false;
				}
				i += 1;
			}
			true
		}
		let bytes = string.as_bytes();

		if bytes_eq(bytes, b"0") {
			return Self(0);
		} else if bytes_eq(bytes, b"air") {
			return Self(1);
		} else if bytes_eq(bytes, b"brick_grey") {
			return Self(2);
		} else if bytes_eq(bytes, b"brick_red") {
			return Self(3);
		} else if bytes_eq(bytes, b"bush") {
			return Self(4);
		} else if bytes_eq(bytes, b"wheat") {
			return Self(5);
		} else if bytes_eq(bytes, b"iron_sword") {
			return Self(6);
		} else if bytes_eq(bytes, b"bow") {
			return Self(7);
		} else if bytes_eq(bytes, b"arrow") {
			return Self(8);
		}
		// add more
		
		Self(0)
	}
}
