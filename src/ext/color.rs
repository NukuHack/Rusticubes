
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}
impl Color {
	pub const DEF_ALPHA: u8 = 255;
	pub const HOVER_ALPHA: u8 = Self::DEF_ALPHA / 2;
	pub const DEF_COLOR: Self = Self::rgba(255, 255, 255, Self::DEF_ALPHA);
	pub const NONE: Self = Self::rgba(0, 0, 0, 0);

	#[inline]pub const fn r(r: u8) -> Self { Self { r, g:0, b:0, a:0 } }
	#[inline]pub const fn g(g: u8) -> Self { Self { r:0, g, b:0, a:0 } }
	#[inline]pub const fn b(b: u8) -> Self { Self { r:0, g:0, b, a:0 } }
	#[inline]pub const fn a(a: u8) -> Self { Self { r:0, g:0, b:0, a } }
	#[inline]pub const fn with_r(self, r: u8) -> Self { Self { r, ..self } }
	#[inline]pub const fn with_g(self, g: u8) -> Self { Self { g, ..self } }
	#[inline]pub const fn with_b(self, b: u8) -> Self { Self { b, ..self } }
	#[inline]pub const fn with_a(self, a: u8) -> Self { Self { a, ..self } }

	#[inline]pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }
	#[inline]pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }
	#[inline]pub const fn tupla((r,g,b,a) : (u8,u8,u8,u8)) -> Self { Self { r, g, b, a } }
	#[inline]pub const fn tupl((r,g,b) : (u8,u8,u8)) -> Self { Self { r, g, b, a:255 } }

	#[inline]pub const fn to_arr(&self) -> [u8;4] { [self.r,self.g,self.b,self.a] }
	#[inline]pub const fn to_tupl(&self) -> (u8,u8,u8,u8) { (self.r,self.g,self.b,self.a) }
	#[inline]pub fn to_vec(&self) -> Vec<u8> { vec![self.r,self.g,self.b,self.a] }

	#[inline]pub const fn o(self) -> Option<Self> { Some(self) }
}
impl std::ops::Mul<f32> for Color {
	type Output = Color;
	fn mul(self, rhs: f32) -> Color {
		// Clamp between 0 and 255
		let clamp = |v: f32| v.max(0.0).min(255.0) as u8;
		Color {
			r: clamp((self.r as f32) * rhs),
			g: clamp((self.g as f32) * rhs),
			b: clamp((self.b as f32) * rhs),
			a: self.a, // Optionally multiply alpha too
		}
	}
}
impl std::ops::Mul<Color> for Color {
	type Output = Color;
	fn mul(self, color: Color) -> Color {
		// Saturating multiplication (clamps at 255)
		Color {
			r: self.r.saturating_mul(color.r),
			g: self.g.saturating_mul(color.g),
			b: self.b.saturating_mul(color.b),
			a: self.a.saturating_mul(color.a),
		}
	}
}
impl std::ops::Add for Color {
	type Output = Color;
	fn add(self, color: Color) -> Color {
		// Saturating addition (clamps at 255)
		Color {
			r: self.r.saturating_add(color.r),
			g: self.g.saturating_add(color.g),
			b: self.b.saturating_add(color.b),
			a: self.a.saturating_add(color.a),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Border {
	pub color: Color,
	pub width: f32,
}
impl Border {
	pub const NONE:Self = Self::rgbaf(0,0,0,0,0.0);

	#[inline]pub const fn rgbaf(r: u8, g: u8, b: u8, a: u8, f:f32) -> Self { Self { color:Color::rgba(r, g, b, a), width:f } }
	#[inline]pub const fn rgbf(r: u8, g: u8, b: u8, f:f32) -> Self { Self { color:Color::rgba(r, g, b, 255), width:f } }
	#[inline]pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Self { color:Color::rgba(r, g, b, a), width:0.0 } }
	#[inline]pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { color:Color::rgba(r, g, b, 255), width:0.0 } }

	#[inline]pub const fn colf(color: Color, f:f32) -> Self { Self { color, width:f } }
	#[inline]pub const fn col(color: Color) -> Self { Self { color, width:0.0 } }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Solor {
	// Basic colors
	Black,
	White,
	Red,
	Green,
	Blue,
	Yellow,
	Cyan,
	Magenta,
	
	// Grayscale
	Gray,
	LightGray,
	DarkGray,
	
	// Web-style colors
	Orange,
	Purple,
	Pink,
	Brown,
	
	// Custom fallback
	Custom(Color),
}

impl Solor {
	pub fn i(self) -> Color{
		self.into()
	}
}

impl From<Solor> for Color {
	fn from(color: Solor) -> Self {
		match color {
			Solor::Black => Color::rgba(0, 0, 0, 255),
			Solor::White => Color::rgba(255, 255, 255, 255),
			Solor::Red => Color::rgba(255, 0, 0, 255),
			Solor::Green => Color::rgba(0, 255, 0, 255),
			Solor::Blue => Color::rgba(0, 0, 255, 255),
			Solor::Yellow => Color::rgba(255, 255, 0, 255),
			Solor::Cyan => Color::rgba(0, 255, 255, 255),
			Solor::Magenta => Color::rgba(255, 0, 255, 255),
			Solor::Gray => Color::rgba(128, 128, 128, 255),
			Solor::LightGray => Color::rgba(200, 200, 200, 255),
			Solor::DarkGray => Color::rgba(80, 80, 80, 255),
			Solor::Orange => Color::rgba(255, 165, 0, 255),
			Solor::Purple => Color::rgba(128, 0, 128, 255),
			Solor::Pink => Color::rgba(255, 192, 203, 255),
			Solor::Brown => Color::rgba(165, 42, 42, 255),
			Solor::Custom(color) => color,
		}
	}
}