
use cgmath::{InnerSpace, Rotation3, Zero};
use wgpu::util::DeviceExt;
use super::instances;

#[allow(dead_code, unused, redundant_imports, unused_results, unused_features, unused_variables, unused_mut, dead_code, unused_unsafe, unused_attributes)]
pub struct InstanceManager {
    pub instances: Vec<Instance>,
    pub instance_data: Vec<InstanceRaw>,
    pub instance_buffer: wgpu::Buffer,
}

impl InstanceManager {
    pub fn new(
        device: &wgpu::Device,
    ) -> Self {
        const SPACE_BETWEEN: f32 = 3.0;
        let instances: Vec<instances::Instance> = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x: f32 = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z: f32 = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position: cgmath::Vector3<f32> = cgmath::Vector3 { x, y: 0.0, z };

                let rotation: cgmath::Quaternion<f32> = if position.is_zero() {
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                instances::Instance {
                    position,
                    rotation,
                }
            })
        }).collect();
        let instance_data: Vec<instances::InstanceRaw> = instances.iter().map(instances::Instance::to_raw).collect();
        let instance_buffer: wgpu::Buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Self {
            instances,
            instance_data,
            instance_buffer,
        }
    }
}

pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn to_raw(&self) -> instances::InstanceRaw {
        instances::InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub const NUM_INSTANCES_PER_ROW: u32 = 10;