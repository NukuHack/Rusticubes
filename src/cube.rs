
use super::geometry::Vertex;
use cgmath::Rotation3;

/// Stores position for X, Y, Z as 4-bit fields: [X:4, Y:4, Z:4, Empty:4]
/// Stores rotations for X, Y, Z as 5-bit fields: [X:5, Y:5, Z:5, Empty:1]
/// Stores 3x3x3 points as a 32-bit "array" [Points: 27, Empty: 5]
#[allow(dead_code, unused)]
#[derive(Clone,Copy)]
pub struct Cube {
    /// in case someone needs it (i do i'm stupid) 4 bits is 0-15 ; 5 bits is 0-32; this goes forever (i think u256 is the current max)
    pub position: u16,    // [X:4, Y:4, Z:4, Empty:4]
    pub material: u16,    // Material info (unused in current implementation)
    pub points: u32,      // 3x3x3 points (27 bits used)
    pub rotation: u16,    // [X:5, Y:5, Z:5, Empty:1]
}

impl Default for Cube {
    /// Creates a new default cube.
     fn default() -> Self {
        Self {
            position: 0,
            material: 1,
            points: 0,
            rotation: 0,
        }
    }
}
impl std::fmt::Debug for Cube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cube")
            .field("position", &format_args!("{:?}", self.position))
            .field("material", &format_args!("{:?}", self.material))
            .field("points", &format_args!("{:?}", self.points))
            .field("rotation", &format_args!("{:?}", self.rotation))
            //.field("vertices", &self.vertices) // constant so i decided not to log it
            //.field("indices", &self.indices) // constant so i decided not to log it
            .finish()
    }
}
#[allow(dead_code, unused)]
impl Cube {
    /// Creates a new cube with a specified position.
    pub fn new(position: u16) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }
    pub fn null()-> Self{
        Self{
            position: 0,
            material: 0,
            points: 0,
            rotation: 0,
        }
    }

    /// Creates a new cube with a specified position and rotation.
    pub fn new_rot(position: u16, rotation: u16) -> Self {
        Self {
            position,
            rotation,
            ..Self::default()
        }
    }

    pub fn new_rot_raw(
        position: cgmath::Vector3<i32>,
        rotation: cgmath::Quaternion<f32>,
    ) -> Self {
        Self {
            position:vector_to_position(position),
            rotation:quaternion_to_rotation(rotation),
            ..Self::default()
        }
    }

    fn get_axis_rotation(&self, axis: char) -> u16 {
        match axis {
            'x' => (self.rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X,
            'y' => (self.rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y,
            'z' => (self.rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z,
            _ => panic!("Invalid axis"),
        }
    }
    /// Extract individual rotation components (0-3)
    pub fn get_x_rotation(&self) -> u16 {
        (self.rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X
    }

    pub fn get_y_rotation(&self) -> u16 {
        (self.rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y
    }

    pub fn get_z_rotation(&self) -> u16 {
        (self.rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z
    }
    /// Rotation snapping and conversion to quaternion
    pub fn rotation_to_quaternion(&self) -> cgmath::Quaternion<f32> {
        let x_rot = self.get_x_rotation() as f32 * (360.0 / 32.0);
        let y_rot = self.get_y_rotation() as f32 * (360.0 / 32.0);
        let z_rot = self.get_z_rotation() as f32 * (360.0 / 32.0);

        let x_q = cgmath::Quaternion::from_angle_x(cgmath::Deg(x_rot));
        let y_q = cgmath::Quaternion::from_angle_y(cgmath::Deg(y_rot));
        let z_q = cgmath::Quaternion::from_angle_z(cgmath::Deg(z_rot));

        z_q * y_q * x_q // Apply rotations in XYZ order
    }

    pub fn rotate(&mut self, axis: char, steps: u16) {
        let current = self.get_axis_rotation(axis);
        let new_rot = (current + steps) % 32; // 5 bits → 0-31
        let (mask, shift) = match axis {
            'x' => (Self::ROT_MASK_X, Self::ROT_SHIFT_X),
            'y' => (Self::ROT_MASK_Y, Self::ROT_SHIFT_Y),
            'z' => (Self::ROT_MASK_Z, Self::ROT_SHIFT_Z),
            _ => unreachable!(),
        };
        self.rotation = (self.rotation & !mask) | ((new_rot as u16) << shift);
        if new_rot == 0 {
            self.reset_points();
        }
    }

    /// Sets the position of the cube in 3D space.
    pub fn set_position(&mut self, x: u16, y: u16, z: u16) {
        self.position = z | (y << 4) | (x << 8);
    }
    /// Convert packed position to world coordinates
    pub fn get_position(&self) -> cgmath::Vector3<i32> {
        let x = ((self.position >> 8) & 0xF) as i32;
        let y = ((self.position >> 4) & 0xF) as i32;
        let z = (self.position & 0xF) as i32;
        cgmath::Vector3::new(x, y, z)
    }
    /// Convert packed position to world coordinates float
    pub fn get_position_f(&self) -> cgmath::Vector3<f32> {
        let x = ((self.position >> 8) & 0xF) as i32;
        let y = ((self.position >> 4) & 0xF) as i32;
        let z = (self.position & 0xF) as i32;
        vec3_i32_to_f32(cgmath::Vector3::new(x, y, z))
    }


    /// Resets the points of the cube when rotation resets.
    fn reset_points(&mut self) {
        self.points = 0;
    }

    /// Generates the mesh points of the cube based on its current state.
    pub fn get_mesh(&self) -> Vec<[u8; 3]> {
        let rotated_positions = self.compute_rotated_positions();
        rotated_positions
            .into_iter()
            .enumerate()
            .filter_map(|(index, [dx, dy, dz])| {
                if (self.points & (1 << index)) != 0 {
                    Some([dx, dy, dz])
                } else {
                    None
                }
            })
            .collect()
    }

    /// Computes the rotated positions for all 27 points based on the current rotation.
    fn compute_rotated_positions(&self) -> [[u8; 3]; 27] {
        let x_rot = self.get_x_rotation() as f32 * (std::f32::consts::PI * 2.0 / 32.0);
        let y_rot = self.get_y_rotation() as f32 * (std::f32::consts::PI * 2.0 / 32.0);
        let z_rot = self.get_z_rotation() as f32 * (std::f32::consts::PI * 2.0 / 32.0);

        let rotation_x = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_x(), cgmath::Rad(x_rot));
        let rotation_y = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Rad(y_rot));
        let rotation_z = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Rad(z_rot));

        let total_rotation = rotation_z * rotation_y * rotation_x; // Apply rotations in XYZ order

        let mut rotated_positions = DOT_ARRAY.to_vec();
        for point in rotated_positions.iter_mut() {
            let vec = cgmath::Vector3::new(
                point[0] as f32 - 1.0, // Center the point (since DOT_ARRAY is offset)
                point[1] as f32 - 1.0,
                point[2] as f32 - 1.0,
            );
            let rotated = total_rotation * vec;
            // Convert back to u8 and clamp to 0-2 range
            point[0] = ((rotated.x + 1.0).clamp(0.0, 2.0) * 3.0).round() as u8 % 3;
            point[1] = ((rotated.y + 1.0).clamp(0.0, 2.0) * 3.0).round() as u8 % 3;
            point[2] = ((rotated.z + 1.0).clamp(0.0, 2.0) * 3.0).round() as u8 % 3;
        }
        rotated_positions.try_into().unwrap()
    }

    /// Rotates the points 90° around the X-axis.
    fn rotate_x(&self, positions: &mut [[u8; 3]]) {
        for point in positions.iter_mut() {
            point.swap(2, 1);
            point[1] = 2 - point[1];
        }
    }

    /// Rotates the points 90° around the Y-axis.
    fn rotate_y(&self, positions: &mut [[u8; 3]]) {
        for point in positions.iter_mut() {
            point.swap(0, 2);
            point[0] = 2 - point[0];
        }
    }

    /// Rotates the points 90° around the Z-axis.
    fn rotate_z(&self, positions: &mut [[u8; 3]]) {
        for point in positions.iter_mut() {
            point.swap(0, 1);
            point[0] = 2 - point[0];
        }
    }

    /// Generates a mesh using the Marching Cubes algorithm.
    pub fn march_cubes(&self) -> Vec<u8> {
        let mut mesh = Vec::new();
        let positions = self.compute_rotated_positions();

        let corners_indices = [0, 2, 6, 8, 18, 20, 24, 26];
        let vertices: [[u8; 3]; 8] = corners_indices
            .iter()
            .map(|&i| positions[i])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = self.calculate_cube_config();
        let triangles = MARCHING_CUBES_TABLE[config as usize];

        let mut current_triangle = Vec::new();
        for &index in triangles.iter() {
            if index == 0 {
                if !current_triangle.is_empty() {
                    mesh.extend(current_triangle.iter().cloned());
                    current_triangle.clear();
                }
                continue;
            }
            current_triangle.push(index);
            if current_triangle.len() == 3 {
                mesh.extend(current_triangle.iter().cloned());
                current_triangle.clear();
            }
        }
        mesh
    }

    /// Calculates the configuration of the cube based on active voxels.
    fn calculate_cube_config(&self) -> u8 {
        let mut config = 0;
        let corners = [0, 2, 6, 8, 18, 20, 24, 26];
        for (i, &index) in corners.iter().enumerate() {
            if (self.points & (1 << index)) != 0 {
                config |= 1 << i;
            }
        }
        config
    }

    pub fn is_empty(&self) -> bool {
        if self.material == 0 {
            true
        } else { false }
    }

    // Full conversion to Instance
    pub fn to_instance(&self) -> super::geometry::Instance {
        let position = self.get_position_f();
        let rotation = self.rotation_to_quaternion();
        
        super::geometry::Instance { position, rotation }
    }
    // Full conversion to Instance
    pub fn to_world_instance(&self, chunk_pos: cgmath::Vector3<i32>) -> super::geometry::Instance {
        let local_position = self.get_position_f();
        let chunk_position = vec3_i32_to_f32(get_chunk_pos(chunk_pos));
        let rotation = self.rotation_to_quaternion();
        
        super::geometry::Instance { position: local_position+chunk_position, rotation }
    }
}

impl Cube {
    /// Rotation masks and shifts for X, Y, Z axes
    const ROT_MASK_X: u16 = 0b11111; // 5 bits for X
    const ROT_SHIFT_X: u32 = 0;

    const ROT_MASK_Y: u16 = 0b11111 << 5; // 5 bits for Y, shifted left by 5
    const ROT_SHIFT_Y: u32 = 5;

    const ROT_MASK_Z: u16 = 0b11111 << 10; // 5 bits for Z, shifted left by 10
    const ROT_SHIFT_Z: u32 = 10;
}

/// Convert a quaternion to the packed u16 rotation format
pub fn quaternion_to_rotation(rotation: cgmath::Quaternion<f32>) -> u16 {
    // Extract components of the quaternion (assuming it's already normalized)
    let w:f32 = rotation.s;
    let x:f32 = rotation.v.x;
    let y:f32 = rotation.v.y;
    let z:f32 = rotation.v.z;

    // Precompute common terms using fused multiply-add where possible
    let xx:f32 = x * x;
    let yy:f32 = y * y;
    let xy:f32 = x * y;
    let yz:f32 = y * z;
    let zx:f32 = z * x;
    
    // Compute angles directly using atan2 and asin
    let pitch:f32 = (2.0 * (w * x + yz)).atan2(1.0 - 2.0 * (xx + yy));
    let yaw:f32 = (2.0 * (w * y - zx)).asin();
    let roll:f32 = (2.0 * (w * z + xy)).atan2(1.0 - 2.0 * (yy + z * z));

    // Normalize and quantize angles in one step using bit operations
    const SCALE: f32 = 31.0 / (2.0 * std::f32::consts::PI);
    
    // Use rem_euclid for normalization and scale directly
    let pitch_bits = ((pitch.rem_euclid(2.0 * std::f32::consts::PI) * SCALE).round() as u16) & 0x1F;
    let yaw_bits = ((yaw.rem_euclid(2.0 * std::f32::consts::PI) * SCALE).round() as u16) & 0x1F;
    let roll_bits = ((roll.rem_euclid(2.0 * std::f32::consts::PI) * SCALE).round() as u16) & 0x1F;

    // Pack into u16: [X:5][Y:5][Z:5][Empty:1]
    pitch_bits | (yaw_bits << 5) | (roll_bits << 10)
}
// Convert world coordinates to packed u16 position
#[inline]
pub fn vector_to_position(position: cgmath::Vector3<i32>) -> u16 {
    ((position.x as u16 & 0xF) << 8) | 
    ((position.y as u16 & 0xF) << 4) | 
    (position.z as u16 & 0xF)
}
// Utility functions for vector type conversion
#[allow(dead_code, unused)]
pub fn vec3_f32_to_u32(v: cgmath::Vector3<f32>) -> cgmath::Vector3<u32> {
    cgmath::Vector3::new(v.x as u32, v.y as u32, v.z as u32)
}

pub fn vec3_f32_to_i32(v: cgmath::Vector3<f32>) -> cgmath::Vector3<i32> {
    cgmath::Vector3::new(v.x as i32, v.y as i32, v.z as i32)
}

#[allow(dead_code, unused)]
pub fn vec3_u32_to_i32(v: cgmath::Vector3<u32>) -> cgmath::Vector3<i32> {
    cgmath::Vector3::new(v.x as i32, v.y as i32, v.z as i32)
}

#[allow(dead_code, unused)]
pub fn vec3_i32_to_f32(v: cgmath::Vector3<i32>) -> cgmath::Vector3<f32> {
    cgmath::Vector3::new(v.x as f32, v.y as f32, v.z as f32)
}

#[allow(dead_code, unused)]
pub fn vec3_i32_to_u32(v: cgmath::Vector3<i32>) -> cgmath::Vector3<u32> {
    cgmath::Vector3::new(v.x as u32, v.y as u32, v.z as u32)
}



pub const DOT_ARRAY: [[u8; 3]; 27] = [
    [2, 0, 0], [2, 0, 1], [2, 0, 2],
    [2, 1, 0], [2, 1, 1], [2, 1, 2],
    [2, 2, 0], [2, 2, 1], [2, 2, 2],

    [1, 0, 0], [1, 0, 1], [1, 0, 2],
    [1, 1, 0], [1, 1, 1], [1, 1, 2],
    [1, 2, 0], [1, 2, 1], [1, 2, 2],

    [0, 0, 0], [0, 0, 1], [0, 0, 2],
    [0, 1, 0], [0, 1, 1], [0, 1, 2],
    [0, 2, 0], [0, 2, 1], [0, 2, 2],
];

pub const MARCHING_CUBES_TABLE: [[u8; 1]; 0] = [
    /* ... */ // Fill in the full table here
];


#[derive(Debug, Clone)]
pub struct Chunk {
    pub position: cgmath::Vector3<i32>,  // World coordinates of chunk (e.g., chunk (x,y,z))
    pub cubes: [Cube; Self::CUBES_PER_CHUNK],  // Array of cubes in the chunk
}

#[allow(dead_code, unused)]
impl Chunk {
    pub const CHUNK_SIZE: usize = 16;
    pub const CHUNK_SIZE_U: u32 = Self::CHUNK_SIZE as u32;
    pub const CHUNK_SIZE_I: i32 = Self::CHUNK_SIZE as i32;
    pub const CUBES_PER_CHUNK: usize = Self::CHUNK_SIZE.pow(3);

    /// Creates a new empty chunk at the specified chunk coordinates
    pub fn null(world_pos: cgmath::Vector3<i32>) -> Self {
        let start = std::time::Instant::now();
        let chunk_pos = Self::world_to_chunk_pos(world_pos);
        // Initialize all cubes as empty
        let cubes = [Cube::null(); Self::CUBES_PER_CHUNK];
        
        //println!("Chunk is being initialized at {:?} - re-corrected chunk pos: {:?}",chunk_pos,chunk_pos);
        println!("null Chunk init took: {:?}", start.elapsed());
        Chunk { 
            position: chunk_pos, 
            cubes 
        }
    }
    /// Creates a new filled chunk at the specified position
    pub fn new(world_pos: cgmath::Vector3<i32>) -> Self {
        let start = std::time::Instant::now();
        let chunk_pos = Self::world_to_chunk_pos(world_pos);
        // Precompute all possible packed positions for this chunk
        let mut precomputed_positions = [[[0u16; Self::CHUNK_SIZE]; Self::CHUNK_SIZE]; Self::CHUNK_SIZE];
        
        for x in 0..Self::CHUNK_SIZE {
            for y in 0..Self::CHUNK_SIZE {
                for z in 0..Self::CHUNK_SIZE {
                    precomputed_positions[x][y][z] = ((x as u16) << 8) | ((y as u16) << 4) | z as u16;
                }
            }
        }

        // Initialize cubes using precomputed positions
        let cubes = std::array::from_fn(|i| {
            let (x, y, z) = (
                (i / (Self::CHUNK_SIZE * Self::CHUNK_SIZE)) % Self::CHUNK_SIZE,
                (i / Self::CHUNK_SIZE) % Self::CHUNK_SIZE,
                i % Self::CHUNK_SIZE
            );
            Cube::new(precomputed_positions[x][y][z])
        });

        //println!("Chunk is being initialized at {:?} - re-corrected chunk pos: {:?}",chunk_pos,chunk_pos);
        println!("basic Chunk init took: {:?}", start.elapsed());
        Chunk { 
            position: chunk_pos,
            cubes
        }
    }

    /// Get cube data at local coordinates (returns None for empty cubes)
    pub fn get(&self, local: cgmath::Vector3<u32>) -> Option<&Cube> {
        let idx = Self::local_to_index(local);
        let cube = &self.cubes[idx as usize];
        if cube.is_empty() {
            None
        } else {
            Some(cube)
        }
    }

    /// Get mutable cube data at local coordinates
    pub fn get_mut(&mut self, local: cgmath::Vector3<u32>) -> Option<&mut Cube> {
        let idx = Self::local_to_index(local);
        let cube = &mut self.cubes[idx as usize];
        if cube.is_empty() {
            None
        } else {
            Some(cube)
        }
    }

    /// Set cube data at local coordinates
    pub fn set(&mut self, local: cgmath::Vector3<u32>, cube: Cube) {
        let idx = Self::local_to_index(local);
        self.cubes[idx as usize] = cube;
    }

    /// Convert world position to chunk coordinates
    pub fn world_to_chunk_pos(world_pos: cgmath::Vector3<i32>) -> cgmath::Vector3<i32> {
        cgmath::Vector3::new(
            world_pos.x.div_euclid(Self::CHUNK_SIZE_I),
            world_pos.y.div_euclid(Self::CHUNK_SIZE_I),
            world_pos.z.div_euclid(Self::CHUNK_SIZE_I),
        )
    }

    /// Convert world position to local chunk coordinates
    pub fn world_to_local_pos(world_pos: cgmath::Vector3<i32>) -> cgmath::Vector3<u32> {
        cgmath::Vector3::new(
            world_pos.x.rem_euclid(Self::CHUNK_SIZE_I) as u32,
            world_pos.y.rem_euclid(Self::CHUNK_SIZE_I) as u32,
            world_pos.z.rem_euclid(Self::CHUNK_SIZE_I) as u32,
        )
    }

    /// Convert local chunk coordinates to world position
    pub fn local_to_world_pos(chunk: Self, local: cgmath::Vector3<u32>) -> cgmath::Vector3<i32> {
        cgmath::Vector3::new(
            chunk.position.x * Self::CHUNK_SIZE_I + local.x as i32,
            chunk.position.y * Self::CHUNK_SIZE_I + local.y as i32,
            chunk.position.z * Self::CHUNK_SIZE_I + local.z as i32,
        )
    }

    /// Convert local coordinates to array index
    #[inline]
    pub fn local_to_index(local: cgmath::Vector3<u32>) -> u32 {
        debug_assert!(local.x < Self::CHUNK_SIZE_U, "Local x coordinate out of bounds");
        debug_assert!(local.y < Self::CHUNK_SIZE_U, "Local y coordinate out of bounds");
        debug_assert!(local.z < Self::CHUNK_SIZE_U, "Local z coordinate out of bounds");
        
        (local.z * Self::CHUNK_SIZE_U * Self::CHUNK_SIZE_U) + 
        (local.y * Self::CHUNK_SIZE_U) + 
        local.x
    }

    /// Convert array index to local coordinates
    #[inline]
    pub fn index_to_local(index: u32) -> cgmath::Vector3<u32> {
        debug_assert!(index < Self::CUBES_PER_CHUNK as u32, "Index out of bounds");
        
        cgmath::Vector3::new(
            index % Self::CHUNK_SIZE_U,
            (index / Self::CHUNK_SIZE_U) % Self::CHUNK_SIZE_U,
            index / (Self::CHUNK_SIZE_U * Self::CHUNK_SIZE_U),
        )
    }

    /// Check if a world position is within this chunk
    pub fn contains_world_pos(&self, world_pos: cgmath::Vector3<i32>) -> bool {
        let chunk_pos = Self::world_to_chunk_pos(world_pos);
        chunk_pos == self.position
    }

    /// Load a chunk at specific chunk coordinates
    pub fn load_chunk(chunk_pos: cgmath::Vector3<i32>) -> Option<Self> {
        Some(Chunk::new(chunk_pos))
    }
}


/// Convert local chunk coordinates to world position
pub fn get_chunk_pos(chunk_pos: cgmath::Vector3<i32>) -> cgmath::Vector3<i32> {
    cgmath::Vector3::new(
        chunk_pos.x * Chunk::CHUNK_SIZE_I,
        chunk_pos.y * Chunk::CHUNK_SIZE_I,
        chunk_pos.z * Chunk::CHUNK_SIZE_I,
    )
}

pub struct CubeBuffer;

impl CubeBuffer {
    pub fn new(device: &wgpu::Device) -> super::geometry::GeometryBuffer {
        super::geometry::GeometryBuffer::new(device, &INDICES, &VERTICES)
    }
}


const VERTICES: [Vertex; 8] = [
    Vertex {
        position: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, 0.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 0.0],
    },
];

const INDICES: [u32; 36] = [
    1, 0, 2, 3, 2, 0, // Front face (z=0)
    4, 5, 6, 6, 7, 4, // Back face (z=-1)
    0, 4, 7, 3, 0, 7, // Bottom (y=0)
    5, 1, 6, 1, 2, 6, // Top (y=1)
    6, 2, 7, 2, 3, 7, // Right (x=1)
    4, 0, 5, 0, 1, 5, // Left (x=0)
];