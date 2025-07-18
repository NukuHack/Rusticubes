#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

impl Color {
	// Constants
	pub const DEF_ALPHA: u8 = 255;
	pub const HOVER_ALPHA: u8 = Self::DEF_ALPHA / 2;
	pub const DEF_COLOR: Self = Self::rgba(255, 255, 255, Self::DEF_ALPHA);
	pub const NONE: Self = Self::rgba(0, 0, 0, 0);
	
	// Common colors
	pub const BLACK: Self = Self::rgb(0, 0, 0);
	pub const WHITE: Self = Self::rgb(255, 255, 255);
	pub const RED: Self = Self::rgb(255, 0, 0);
	pub const GREEN: Self = Self::rgb(0, 255, 0);
	pub const BLUE: Self = Self::rgb(0, 0, 255);
	pub const YELLOW: Self = Self::rgb(255, 255, 0);
	pub const MAGENTA: Self = Self::rgb(255, 0, 255);
	pub const CYAN: Self = Self::rgb(0, 255, 255);
	pub const GRAY: Self = Self::rgb(128, 128, 128);
	pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

	// Constructors
	#[inline] pub const fn r(r: u8) -> Self { Self { r, g:0, b:0, a:0 } }
	#[inline] pub const fn g(g: u8) -> Self { Self { r:0, g, b:0, a:0 } }
	#[inline] pub const fn b(b: u8) -> Self { Self { r:0, g:0, b, a:0 } }
	#[inline] pub const fn a(a: u8) -> Self { Self { r:0, g:0, b:0, a } }
	#[inline] pub const fn with_r(self, r: u8) -> Self { Self { r, ..self } }
	#[inline] pub const fn with_g(self, g: u8) -> Self { Self { g, ..self } }
	#[inline] pub const fn with_b(self, b: u8) -> Self { Self { b, ..self } }
	#[inline] pub const fn with_a(self, a: u8) -> Self { Self { a, ..self } }

	#[inline] pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }
	#[inline] pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }
	#[inline] pub const fn tupla((r,g,b,a): (u8,u8,u8,u8)) -> Self { Self { r, g, b, a } }
	#[inline] pub const fn tupl((r,g,b): (u8,u8,u8)) -> Self { Self { r, g, b, a:255 } }

	// Conversion methods
	#[inline] pub const fn to_arr(&self) -> [u8;4] { [self.r,self.g,self.b,self.a] }
	#[inline] pub const fn to_tupl(&self) -> (u8,u8,u8,u8) { (self.r,self.g,self.b,self.a) }
	#[inline] pub fn to_vec(&self) -> Vec<u8> { vec![self.r,self.g,self.b,self.a] }
	#[inline] pub const fn o(self) -> Option<Self> { Some(self) }

	// Color operations
	pub fn lerp(self, other: Self, t: f32) -> Self {
		let t = t.clamp(0.0, 1.0);
		Self {
			r: lerp_u8(self.r, other.r, t),
			g: lerp_u8(self.g, other.g, t),
			b: lerp_u8(self.b, other.b, t),
			a: lerp_u8(self.a, other.a, t),
		}
	}

	pub fn lerp_alpha(self, other: Self, t: f32) -> Self {
		let t = t.clamp(0.0, 1.0);
		Self {
			r: self.r,
			g: self.g,
			b: self.b,
			a: lerp_u8(self.a, other.a, t),
		}
	}

	pub fn grayscale(&self) -> Self {
		let gray = ((self.r as f32 * 0.299) + (self.g as f32 * 0.587) + (self.b as f32 * 0.114)) as u8;
		Self::rgba(gray, gray, gray, self.a)
	}

	pub fn invert(&self) -> Self {
		Self {
			r: 255 - self.r,
			g: 255 - self.g,
			b: 255 - self.b,
			a: self.a,
		}
	}

	pub fn blend_normal(&self, other: Self) -> Self {
		if other.a == 0 {
			return *self;
		}
		if other.a == 255 {
			return other;
		}

		let alpha = other.a as f32 / 255.0;
		let inv_alpha = 1.0 - alpha;

		Self {
			r: ((self.r as f32 * inv_alpha) + (other.r as f32 * alpha)) as u8,
			g: ((self.g as f32 * inv_alpha) + (other.g as f32 * alpha)) as u8,
			b: ((self.b as f32 * inv_alpha) + (other.b as f32 * alpha)) as u8,
			a: self.a.saturating_add(other.a),
		}
	}

	pub fn blend_multiply(&self, other: Self) -> Self {
		Self {
			r: ((self.r as u16 * other.r as u16) / 255) as u8,
			g: ((self.g as u16 * other.g as u16) / 255) as u8,
			b: ((self.b as u16 * other.b as u16) / 255) as u8,
			a: ((self.a as u16 * other.a as u16) / 255) as u8,
		}
	}

	pub fn blend_additive(&self, other: Self) -> Self {
		Self {
			r: self.r.saturating_add(other.r),
			g: self.g.saturating_add(other.g),
			b: self.b.saturating_add(other.b),
			a: self.a.saturating_add(other.a),
		}
	}

	pub fn blend_screen(&self, other: Self) -> Self {
		Self {
			r: 255 - (((255 - self.r as u16) * (255 - other.r as u16)) / 255) as u8,
			g: 255 - (((255 - self.g as u16) * (255 - other.g as u16)) / 255) as u8,
			b: 255 - (((255 - self.b as u16) * (255 - other.b as u16)) / 255) as u8,
			a: self.a.saturating_add(other.a),
		}
	}

	pub fn brightness(&self) -> f32 {
		(self.r as f32 * 0.299 + self.g as f32 * 0.587 + self.b as f32 * 0.114) / 255.0
	}

	pub fn set_brightness(&self, brightness: f32) -> Self {
		let brightness = brightness.clamp(0.0, 1.0);
		let current = self.brightness();
		if current == 0.0 {
			return Self::rgb(
				(brightness * 255.0) as u8,
				(brightness * 255.0) as u8,
				(brightness * 255.0) as u8,
			);
		}
		let factor = brightness / current;
		Self {
			r: ((self.r as f32 * factor).clamp(0.0, 255.0) as u8),
			g: ((self.g as f32 * factor).clamp(0.0, 255.0) as u8),
			b: ((self.b as f32 * factor).clamp(0.0, 255.0) as u8),
			a: self.a,
		}
	}

	pub fn to_hex(&self) -> String {
		format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
	}

	pub fn from_hex(hex: &str) -> Option<Self> {
		let hex = hex.trim_start_matches('#');
		match hex.len() {
			3 => {
				let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
				let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
				let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
				Some(Self::rgb(r, g, b))
			}
			6 => {
				let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
				let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
				let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
				Some(Self::rgb(r, g, b))
			}
			8 => {
				let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
				let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
				let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
				let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
				Some(Self::rgba(r, g, b, a))
			}
			_ => None,
		}
	}

	pub fn to_hsl(&self) -> (f32, f32, f32) {
		let r = self.r as f32 / 255.0;
		let g = self.g as f32 / 255.0;
		let b = self.b as f32 / 255.0;

		let max = r.max(g.max(b));
		let min = r.min(g.min(b));
		let mut h = 0.0;
		let s: f32;
		let l = (max + min) / 2.0;

		if max == min {
			h = 0.0;
			s = 0.0;
		} else {
			let d = max - min;
			s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

			match max {
				x if x == r => h = (g - b) / d + (if g < b { 6.0 } else { 0.0 }),
				x if x == g => h = (b - r) / d + 2.0,
				x if x == b => h = (r - g) / d + 4.0,
				_ => (),
			}

			h /= 6.0;
		}

		(h, s, l)
	}

	pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
		let h = h.clamp(0.0, 1.0);
		let s = s.clamp(0.0, 1.0);
		let l = l.clamp(0.0, 1.0);

		let r: f32;
		let g: f32;
		let b: f32;

		if s == 0.0 {
			r = l;
			g = l;
			b = l;
		} else {
			let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
			let p = 2.0 * l - q;

			r = hue_to_rgb(p, q, h + 1.0/3.0);
			g = hue_to_rgb(p, q, h);
			b = hue_to_rgb(p, q, h - 1.0/3.0);
		}

		Self::rgb(
			(r * 255.0).round() as u8,
			(g * 255.0).round() as u8,
			(b * 255.0).round() as u8,
		)
	}

	pub fn with_hue(&self, hue: f32) -> Self {
		let (_, s, l) = self.to_hsl();
		Self::from_hsl(hue, s, l)
	}

	pub fn with_saturation(&self, saturation: f32) -> Self {
		let (h, _, l) = self.to_hsl();
		Self::from_hsl(h, saturation, l)
	}

	pub fn with_lightness(&self, lightness: f32) -> Self {
		let (h, s, _) = self.to_hsl();
		Self::from_hsl(h, s, lightness)
	}

	pub fn premultiply_alpha(&self) -> Self {
		let alpha = self.a as f32 / 255.0;
		Self {
			r: ((self.r as f32 * alpha).round() as u8),
			g: ((self.g as f32 * alpha).round() as u8),
			b: ((self.b as f32 * alpha).round() as u8),
			a: self.a,
		}
	}

	pub fn darken(&self, amount: f32) -> Self {
		let amount = amount.clamp(0.0, 1.0);
		Self {
			r: ((self.r as f32 * (1.0 - amount)).clamp(0.0, 255.0)) as u8,
			g: ((self.g as f32 * (1.0 - amount)).clamp(0.0, 255.0)) as u8,
			b: ((self.b as f32 * (1.0 - amount)).clamp(0.0, 255.0)) as u8,
			a: self.a,
		}
	}

	pub fn lighten(&self, amount: f32) -> Self {
		let amount = amount.clamp(0.0, 1.0);
		Self {
			r: ((self.r as f32 + (255.0 - self.r as f32) * amount).clamp(0.0, 255.0)) as u8,
			g: ((self.g as f32 + (255.0 - self.g as f32) * amount).clamp(0.0, 255.0)) as u8,
			b: ((self.b as f32 + (255.0 - self.b as f32) * amount).clamp(0.0, 255.0)) as u8,
			a: self.a,
		}
	}
}
impl std::ops::Mul<f32> for Color {
	type Output = Color;
	fn mul(self, rhs: f32) -> Color {
		let clamp = |v: f32| v.max(0.0).min(255.0) as u8;
		Color {
			r: clamp((self.r as f32) * rhs),
			g: clamp((self.g as f32) * rhs),
			b: clamp((self.b as f32) * rhs),
			a: self.a,
		}
	}
}
impl std::ops::Mul<Color> for Color {
	type Output = Color;
	fn mul(self, color: Color) -> Color {
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
		Color {
			r: self.r.saturating_add(color.r),
			g: self.g.saturating_add(color.g),
			b: self.b.saturating_add(color.b),
			a: self.a.saturating_add(color.a),
		}
	}
}
impl std::ops::Sub for Color {
	type Output = Color;
	fn sub(self, color: Color) -> Color {
		Color {
			r: self.r.saturating_sub(color.r),
			g: self.g.saturating_sub(color.g),
			b: self.b.saturating_sub(color.b),
			a: self.a.saturating_sub(color.a),
		}
	}
}
impl std::ops::AddAssign for Color {
	fn add_assign(&mut self, rhs: Self) {
		*self = *self + rhs;
	}
}
impl std::ops::SubAssign for Color {
	fn sub_assign(&mut self, rhs: Self) {
		*self = *self - rhs;
	}
}
impl std::ops::MulAssign<f32> for Color {
	fn mul_assign(&mut self, rhs: f32) {
		*self = *self * rhs;
	}
}
impl std::ops::MulAssign<Color> for Color {
	fn mul_assign(&mut self, rhs: Color) {
		*self = *self * rhs;
	}
}
// Helper functions
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
	((a as f32) * (1.0 - t) + (b as f32) * t).round() as u8
}
fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
	let mut t = t;
	if t < 0.0 { t += 1.0; }
	if t > 1.0 { t -= 1.0; }
	
	if t < 1.0/6.0 { return p + (q - p) * 6.0 * t; }
	if t < 1.0/2.0 { return q; }
	if t < 2.0/3.0 { return p + (q - p) * (2.0/3.0 - t) * 6.0; }
	p
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