
use std::ops::Mul;

#[derive(Debug, Clone, Copy)]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}
impl Color {
	pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }
	pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }

	pub const fn get_rgb(&self) -> (u8,u8,u8) {(self.r,self.g,self.b)}
	pub const fn get_rgbb(&self) -> (u8,u8,u8,u8) {(self.r,self.g,self.b,self.a)}
}
impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Color {
        // Clamp the multiplier between 0.0 and 1.0 to prevent overflow
        let multiplier = rhs.max(0.0).min(1.0);
        
        Color {
            r: ((self.r as f32) * multiplier) as u8,
            g: ((self.g as f32) * multiplier) as u8,
            b: ((self.b as f32) * multiplier) as u8,
            a: self.a, // You might want to multiply alpha too, or leave it as is
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Border {
	pub color: Color,
	pub width: f32,
}
impl Border {
	pub const fn rgbaf(r: u8, g: u8, b: u8, a: u8, f:f32) -> Self { Self { color:Color::rgba(r, g, b, a),width:f } }
	pub const fn rgbf(r: u8, g: u8, b: u8, f:f32) -> Self { Self { color:Color::rgb(r, g, b),width:f } }
	pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Self { color:Color::rgba(r, g, b, a),width:0.0 } }
	pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { color:Color::rgb(r, g, b),width:0.0 } }

	pub const fn col(color: Color) -> Self { Self { color,width:0.0 } }
	pub const fn colf(color: Color, f:f32) -> Self { Self { color,width:f } }
}