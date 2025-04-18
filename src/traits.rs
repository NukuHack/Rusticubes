#[allow(dead_code, unused)]
pub trait PositionConversion {
    fn to_point(&self) -> cgmath::Point3<f32>;
    fn to_vector(&self) -> cgmath::Vector3<f32>;
}

impl PositionConversion for cgmath::Vector3<f32> {
    fn to_point(&self) -> cgmath::Point3<f32> {
        cgmath::Point3::new(self.x, self.y, self.z)
    }

    fn to_vector(&self) -> cgmath::Vector3<f32> {
        *self
    }
}

impl PositionConversion for cgmath::Point3<f32> {
    fn to_point(&self) -> cgmath::Point3<f32> {
        *self
    }

    fn to_vector(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(self.x, self.y, self.z)
    }
}
