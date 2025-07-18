use cgmath::Vector3;
use wgpu::util::DeviceExt;
use std::mem;

// --- Vertex Structure (unchanged) ---
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// --- Simplified Instance Data (position only) ---
#[derive(Clone)]
pub struct Instance {
    pub position: Vector3<f32>,
}

impl Instance {
    pub fn new(position: Vector3<f32>) -> Self {
        Self { position }
    }
    
    // Convert to raw data for GPU (just position)
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            position: [self.position.x, self.position.y, self.position.z],
        }
    }
}

// --- Raw Instance Data (sent to GPU) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub position: [f32; 3],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance, // This marks it as instance data
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// --- Instance Manager ---
pub struct InstanceManager {
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device, instances: Vec<Instance>) -> Self {
        let instance_data: Vec<InstanceRaw> = instances.iter().map(|i| i.to_raw()).collect();
        
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        Self { instances, instance_buffer }
    }
    
    pub fn update_buffer(&self, queue: &wgpu::Queue) {
        let instance_data: Vec<InstanceRaw> = self.instances.iter().map(|i| i.to_raw()).collect();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));
    }
}

// --- Simplified Shader ---
const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct InstanceInput {
    @location(5) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // Simple translation
    let world_position = vec4<f32>( model.position + instance.position, 1.0);

    var out: VertexOutput;
        
    out.clip_position = camera.view_proj * world_position;
    out.uv = model.uv;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.uv.x, in.uv.y, 0.5, 1.0);
}
"#;



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




// AI explain v.2: 




# Instanced Rendering Explained (Simplified)

## How It Works Step-by-Step

1. **Create Your Geometry**: Make a vertex buffer with your mesh data (cube, sphere, etc.)

2. **Create Instances**: Make a list of positions where you want copies to appear
   ```rust
   let instances = vec![
       Instance::new(Vector3::new(0.0, 0.0, 0.0)),
       Instance::new(Vector3::new(2.0, 0.0, 0.0)),
       // ... more positions
   ];
   ```

3. **Create Instance Buffer**: Upload these positions to GPU
   ```rust
   let instance_manager = InstanceManager::new(&device, instances);
   ```

4. **Render**: In your render pass:
   ```rust
   render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
   render_pass.set_vertex_buffer(1, instance_manager.instance_buffer.slice(..));
   render_pass.draw_indexed(0..num_indices, 0, 0..instance_count as u32);
   ```

## Key Differences From Regular Rendering

1. **Two Vertex Buffers**: 
   - Buffer 0: Vertex data (shape)
   - Buffer 1: Instance data (positions)

2. **Shader Inputs**:
   - Regular attributes (location 0-2): Per-vertex data
   - Instance attributes (location 5-8): Per-instance data (changes once per object)

3. **Single Draw Call**: Draws all instances at once with `draw_indexed`

## Performance Benefits

- **Reduced CPU-GPU communication**: One call instead of many
- **GPU optimization**: Can process instances in parallel
- **Memory efficient**: Reuses the same vertex data

## When to Use Instanced Rendering

- Many copies of the same object (trees, bullets, particles)
- Objects with the same geometry but different positions
- When you need to render thousands of objects efficiently

The simplified version removes rotation since you mentioned you don't need it, making the code cleaner while maintaining all the performance benefits of instanced rendering.



*/