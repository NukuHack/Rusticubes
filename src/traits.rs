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

#[allow(dead_code, unused)]
pub trait VectorTypeConversion {
    fn to_vec3_i32(&self) -> cgmath::Vector3<i32>;
    fn to_vec3_u32(&self) -> cgmath::Vector3<u32>;
    fn to_vec3_f32(&self) -> cgmath::Vector3<f32>;
}
impl VectorTypeConversion for cgmath::Vector3<u32> {
    #[inline]
    fn to_vec3_i32(&self) -> cgmath::Vector3<i32> {
        cgmath::Vector3::new(self.x as i32, self.y as i32, self.z as i32)
    }

    #[inline]
    fn to_vec3_u32(&self) -> cgmath::Vector3<u32> {
        *self
    }

    #[inline]
    fn to_vec3_f32(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(self.x as f32, self.y as f32, self.z as f32)
    }
}
impl VectorTypeConversion for cgmath::Vector3<i32> {
    #[inline]
    fn to_vec3_i32(&self) -> cgmath::Vector3<i32> {
        *self
    }

    #[inline]
    fn to_vec3_u32(&self) -> cgmath::Vector3<u32> {
        cgmath::Vector3::new(self.x as u32, self.y as u32, self.z as u32)
    }

    #[inline]
    fn to_vec3_f32(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(self.x as f32, self.y as f32, self.z as f32)
    }
}
impl VectorTypeConversion for cgmath::Vector3<f32> {
    #[inline]
    fn to_vec3_i32(&self) -> cgmath::Vector3<i32> {
        cgmath::Vector3::new(self.x as i32, self.y as i32, self.z as i32)
    }

    #[inline]
    fn to_vec3_u32(&self) -> cgmath::Vector3<u32> {
        cgmath::Vector3::new(self.x as u32, self.y as u32, self.z as u32)
    }

    #[inline]
    fn to_vec3_f32(&self) -> cgmath::Vector3<f32> {
        *self
    }
}
