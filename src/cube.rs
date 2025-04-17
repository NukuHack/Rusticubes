use crate::geometry::Vertex;

/// Stores position for X, Y, Z as 4-bit fields: [X:4, Y:4, Z:4, Empty:4]
/// Stores rotations for X, Y, Z as 5-bit fields: [X:5, Y:5, Z:5, Empty:1]
/// Stores 3x3x3 points as a 32-bit "array" [Points: 27, Empty: 5]
#[allow(dead_code, unused)]
#[derive(Debug, Clone)]
pub struct Cube {
    /// in case someone needs it (i do i'm stupid) 4 bits is 0-15 ; 5 bits is 0-32; this goes forever (i think u256 is the current max)
    pub position: u16,    // [X:4, Y:4, Z:4, Empty:4]
    pub material: u16,    // Material info (unused in current implementation)
    pub points: u32,      // 3x3x3 points (27 bits used)
    pub rotation: u16,    // [X:5, Y:5, Z:5, Empty:1]
    pub vertices: [Vertex; 8],
    pub indices: [u32; 36],
}

#[allow(dead_code, unused)]
impl Cube {
    /// Creates a new default cube.
    pub fn default() -> Self {
        Self {
            position: 0,
            material: 1,
            points: 0,
            rotation: 0,
            vertices: Self::VERTICES,
            indices: Self::INDICES,
        }
    }

    /// Creates a new cube with a specified position.
    pub fn new(position: u16) -> Self {
        Self {
            position,
            ..Self::default()
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

    fn get_axis_rotation(&self, axis: char) -> u16 {
        match axis {
            'x' => (self.rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X,
            'y' => (self.rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y,
            'z' => (self.rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z,
            _ => panic!("Invalid axis"),
        }
    }

    pub fn rotate(&mut self, axis: char, steps: u16) {
        let current = self.get_axis_rotation(axis);
        let new_rot = (current + steps) % 4; // Keep modulo 4 for compatibility

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
        let mut rotated_positions = DOT_ARRAY.to_vec();

        let x_rot = self.get_axis_rotation('x');
        for _ in 0..x_rot {
            self.rotate_x(&mut rotated_positions);
        }

        let y_rot = self.get_axis_rotation('y');
        for _ in 0..y_rot {
            self.rotate_y(&mut rotated_positions);
        }

        let z_rot = self.get_axis_rotation('z');
        for _ in 0..z_rot {
            self.rotate_z(&mut rotated_positions);
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
}

impl Cube {
    /// Rotation masks and shifts for X, Y, Z axes
    const ROT_MASK_X: u16 = 0b11111; // 5 bits for X
    const ROT_SHIFT_X: u32 = 0;

    const ROT_MASK_Y: u16 = 0b11111 << 5; // 5 bits for Y, shifted left by 5
    const ROT_SHIFT_Y: u32 = 5;

    const ROT_MASK_Z: u16 = 0b11111 << 10; // 5 bits for Z, shifted left by 10
    const ROT_SHIFT_Z: u32 = 10;

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
    blocks: Vec<Option<Cube>>, // `None` represents air blocks
}

#[allow(dead_code, unused)]
impl Chunk {
    pub fn new() -> Self {
        Chunk {
            blocks: vec![None; Self::CHUNK_SIZE * Self::CHUNK_SIZE * Self::CHUNK_SIZE],
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<&Cube> {
        self.blocks[z * Self::CHUNK_SIZE * Self::CHUNK_SIZE + y * Self::CHUNK_SIZE + x].as_ref()
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block: Cube) {
        self.blocks[z * Self::CHUNK_SIZE * Self::CHUNK_SIZE + y * Self::CHUNK_SIZE + x] = Some(block);
    }
}

impl Chunk {
    const CHUNK_SIZE: usize = 16;
}

pub struct CubeBuffer;

impl CubeBuffer {
    pub fn new(device: &wgpu::Device, cube: &super::cube::Cube) -> super::geometry::GeometryBuffer {
        super::geometry::GeometryBuffer::new(device, &cube.indices, &cube.vertices)
    }
}