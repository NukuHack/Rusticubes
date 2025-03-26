
use super::cube;

/// Stores position for X, Y, Z as 4-bit fields: [X:4, Y:4, Z:4, Empty: 4]
/// Stores rotations for X, Y, Z as 2-bit fields: [X:2, Y:2, Z:2, Empty: 2]
/// Stores 3*3*3 points as a 32 bit "array" [Points: 3*3*3->27, Empty: 5]
pub struct Cube {
    pub position: u16,
    pub material: u16,
    pub points: u32,
    pub rotation: u8,
}

impl Cube {
    /// Returns whether the cube is "nice" (placeholder for future functionality).
    fn nice() -> bool {
        true
    }

    /// Creates a new default cube.
    pub fn default() -> Self {
        Self {
            position: 0,
            material: 1,
            points: 0,
            rotation: 0, // All rotations initialized to 0
        }
    }


    /// Gets the current rotation for a specific axis.
    fn get_axis_rotation(&self, axis: char) -> u8 {
        match axis {
            'x' => (self.rotation >> 6) & 0b11,
            'y' => (self.rotation >> 4) & 0b11,
            'z' => (self.rotation >> 2) & 0b11,
            _ => panic!("Invalid axis: {}", axis),
        }
    }

    pub fn rotate(&mut self, axis: char, steps: u8) {
        let current_rotation = self.get_axis_rotation(axis);
        let new_rotation = (current_rotation + steps) % 4;

        match axis {
            'x' => {
                self.rotation = (new_rotation << 6) | (self.rotation & 0x3F);
            },
            'y' => {
                self.rotation = (new_rotation << 4) | (self.rotation & 0xCF);
            },
            'z' => {
                self.rotation = (new_rotation << 2) | (self.rotation & 0xF3);
            },
            _ => panic!("Invalid axis: {}", axis),
        }

        if new_rotation == 0 {
            self.reset_points();
        }
    }

    /// Sets the position of the cube in 3D space.
    pub fn set_position(&mut self, x: u8, y: u8, z: u8) {
        self.position = (z as u16) | ((y as u16) << 4) | ((x as u16) << 8);
    }

    /// Resets the points of the cube when rotation resets.
    fn reset_points(&mut self) {
        self.points = 0;
    }


    /// Generates the mesh points of the cube based on its current state.
    pub fn get_mesh(&self) -> Vec<[u8; 3]> {
        // Precomputed rotated positions for all 27 points
        let rotated_positions = self.compute_rotated_positions();

        // Collect only the points that are "full" (1)
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
            point[1] = 2 - &point[1];
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

        // Extract 8 corners
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


// Define the Marching Cubes lookup table
pub const MARCHING_CUBES_TABLE: [[u8; 1];0] = [
    /* ... */ // Fill in the full table here
];