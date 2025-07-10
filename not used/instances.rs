use cgmath::{Matrix4, Vector3, Quaternion, Deg, Rotation3, SquareMatrix};
use wgpu::util::DeviceExt;
use std::mem;

// --- Basic Vertex Structure ---
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // UV
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// --- Instance Data (what makes each instance unique) ---
#[derive(Clone)]
pub struct Instance {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
}

impl Instance {
    pub fn new(position: Vector3<f32>, rotation: Quaternion<f32>) -> Self {
        Self { position, rotation }
    }
    
    // Convert to raw data for GPU
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)).into(),
        }
    }
}

// --- Raw Instance Data (sent to GPU) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4], // 4x4 transformation matrix
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance, // This is the key difference!
            attributes: &[
                // Matrix column 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Matrix column 1
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Matrix column 2
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Matrix column 3
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// --- Simple Instance Manager ---
pub struct InstanceManager {
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, instances: Vec<Instance>) -> Self {
        // Create buffer with instance data
        let instance_data: Vec<InstanceRaw> = instances.iter().map(|i| i.to_raw()).collect();
        
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        Self {
            instances,
            instance_buffer,
        }
    }
    
    pub fn update_buffer(&self, queue: &wgpu::Queue) {
        let instance_data: Vec<InstanceRaw> = self.instances.iter().map(|i| i.to_raw()).collect();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));
    }
}

// --- Simple Geometry Buffer ---
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl GeometryBuffer {
    pub fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }
}

// --- Example Usage ---
pub fn create_cube_instances() -> Vec<Instance> {
    let mut instances = Vec::new();
    
    // Create a 3x3 grid of cubes
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                let position = Vector3::new(
                    x as f32 * 2.0 - 2.0,
                    y as f32 * 2.0 - 2.0,
                    z as f32 * 2.0 - 2.0,
                );
                let rotation = Quaternion::from_angle_y(Deg(x as f32 * 45.0));
                instances.push(Instance::new(position, rotation));
            }
        }
    }
    
    instances
}

// --- Rendering Function ---
pub fn render_instanced(
    render_pass: &mut wgpu::RenderPass,
    pipeline: &wgpu::RenderPipeline,
    geometry: &GeometryBuffer,
    instances: &InstanceManager,
    bind_groups: &[&wgpu::BindGroup],
) {
    // Set the pipeline
    render_pass.set_pipeline(pipeline);
    
    // Set bind groups (textures, uniforms, etc.)
    for (i, bind_group) in bind_groups.iter().enumerate() {
        render_pass.set_bind_group(i as u32, bind_group, &[]);
    }
    
    // Set vertex buffers
    render_pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));      // Vertex data
    render_pass.set_vertex_buffer(1, instances.instance_buffer.slice(..));   // Instance data
    
    // Set index buffer
    render_pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    
    // Draw all instances in one call!
    render_pass.draw_indexed(
        0..geometry.num_indices,           // Index range
        0,                                 // Base vertex
        0..instances.instances.len() as u32 // Instance range
    );
}

// --- Pipeline Creation Helper ---
pub fn create_instanced_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Instanced Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[
                Vertex::desc(),      // Vertex buffer layout
                InstanceRaw::desc(), // Instance buffer layout
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}



const SHADER: str = "

// Vertex shader inputs
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// Instance data (transformation matrix)
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

// Camera uniform
struct Camera {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // Reconstruct the model matrix from instance data
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    
    // Transform vertex position
    let world_position = model_matrix * vec4<f32>(model.position, 1.0);
    
    // Transform normal (assuming uniform scaling)
    let world_normal = (model_matrix * vec4<f32>(model.normal, 0.0)).xyz;
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = normalize(world_normal);
    out.uv = model.uv;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting calculation
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let light_strength = max(dot(in.world_normal, light_dir), 0.1);
    
    // Simple color based on UV coordinates
    let color = vec3<f32>(in.uv.x, in.uv.y, 0.5);
    
    return vec4<f32>(color * light_strength, 1.0);
}

";



/*
// AI explain: 

## Key Concepts:

1. **Instance Data**: Each instance has its own transformation matrix (position + rotation)
2. **Instance Buffer**: Contains transformation matrices for ALL instances
3. **Vertex Step Mode**: The crucial difference - `VertexStepMode::Instance` vs `VertexStepMode::Vertex`

## How It Works:

1. **Vertex Buffer**: Contains the geometry data (one cube's vertices)
2. **Instance Buffer**: Contains transformation matrices (one per cube instance)
3. **Draw Call**: `draw_indexed(0..indices, 0, 0..instance_count)` - renders the same geometry multiple times with different transforms

## The Magic:

- **Vertex attributes (locations 0-2)**: Read from vertex buffer, advance per vertex
- **Instance attributes (locations 5-8)**: Read from instance buffer, advance per instance
- **GPU work**: It automatically uses the same vertex data but different instance data for each copy

## Simple Example Usage:

```rust
// Create 27 cubes in a 3x3x3 grid
let instances = create_cube_instances();
let instance_manager = InstanceManager::new(device, queue, instances);

// In your render loop:
render_instanced(
    &mut render_pass,
    &pipeline,
    &geometry_buffer,  // One cube's worth of vertices
    &instance_manager, // 27 different transformation matrices
    &bind_groups,
);
```

This renders 27 cubes with just **one draw call** instead of 27 separate draw calls!

The key insight is that the GPU reads vertex data normally, but for instance data, it only advances to the next instance when starting a new copy of the geometry.


*/