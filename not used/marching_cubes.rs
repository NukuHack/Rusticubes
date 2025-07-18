

use crate::block::lut::{EDGE_TABLE, TRI_TABLE};

/// Builder for constructing chunk meshes efficiently
pub struct ChunkMeshBuilder {
	pub vertices: Vec<Vertex>,
	pub indices: Vec<u16>,
	current_vertex: u32,
}

impl Default for ChunkMeshBuilder {
	#[inline]
	fn default() -> Self {
		Self::new()
	}
}

impl ChunkMeshBuilder {
	/// Generates a marching cubes mesh for the given block
	pub fn add_marching_cube(&mut self, position: Vec3, points: u32) {
		if points == 0 || points == 0xFF_FF_FF_FF {
			return; // Early exit for empty or full cubes
		}

		let base_pos = position;
		let mut edge_vertex_cache = [None; 12];

		// Process each of the 8 sub-cubes
		for i in 0..8 {
			let case_index = Self::calculate_case_index(points, i);

			// Skip empty or full sub-cubes
			if case_index == 0 || case_index == 255 {
				continue;
			}

			let edges = EDGE_TABLE[case_index as usize];
			if edges == 0 {
				continue;
			}

			// Calculate sub-cube offset
			let sub_offset = Vec3::new(
				(i & 1) as f32 * 0.5,
				((i >> 1) & 1) as f32 * 0.5,
				((i >> 2) & 1) as f32 * 0.5,
			);

			// Cache edge vertices
			for edge in 0..12 {
				if (edges & (1 << edge)) != 0 && edge_vertex_cache[edge].is_none() {
					let [a, b] = EDGE_VERTICES[edge];
					edge_vertex_cache[edge] = Some(a.lerp(b, 0.5));
				}
			}

			// Generate triangles
			self.generate_triangles(case_index, &edge_vertex_cache, base_pos + sub_offset);

			// Clear cache for next sub-cube
			edge_vertex_cache = [None; 12];
		}
	}
	#[inline]
	fn calculate_case_index(points: u32, sub_cube_idx: usize) -> u8 {
		let mut case_index = 0u8;
		for bit in 0..8 {
			let (x, y, z) = match bit {
				0 => (0, 0, 0),
				1 => (1, 0, 0),
				2 => (1, 0, 1),
				3 => (0, 0, 1),
				4 => (0, 1, 0),
				5 => (1, 1, 0),
				6 => (1, 1, 1),
				7 => (0, 1, 1),
				_ => unreachable!(),
			};

			let x = x + ((sub_cube_idx & 1) as u8);
			let y = y + (((sub_cube_idx >> 1) & 1) as u8);
			let z = z + (((sub_cube_idx >> 2) & 1) as u8);

			let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
			if (points & (1u32 << bit_pos)) != 0 {
				case_index |= 1 << bit;
			}
		}
		case_index
	}
	#[inline]
	fn generate_triangles(
		&mut self,
		case_index: u8,
		edge_vertex_cache: &[Option<Vec3>; 12],
		position: Vec3,
	) {
		let triangles = &TRI_TABLE[case_index as usize];
		let mut i = 0;

		while i < 16 && triangles[i] != -1 {
			let v0 = edge_vertex_cache[triangles[i] as usize].unwrap();
			let v1 = edge_vertex_cache[triangles[i + 1] as usize].unwrap();
			let v2 = edge_vertex_cache[triangles[i + 2] as usize].unwrap();

			self.add_triangle(&[position + v0, position + v1, position + v2]);
			i += 3;
		}
	}
	/// Adds a triangle to the mesh with calculated normals
	#[inline]
	pub fn add_triangle(&mut self, vertices: &[Vec3; 3]) {
		// Calculate face normal
		let edge1 = vertices[1] - vertices[0];
		let edge2 = vertices[2] - vertices[0];
		let normal = edge1.cross(edge2).normalize();
		let normal_arr = [normal.x, normal.y, normal.z];

		let base = self.current_vertex as u16;
		self.vertices.extend([
			Vertex {
				position: [vertices[0].x, vertices[0].y, vertices[0].z],
				normal: normal_arr,
				uv: [0f32, 0f32],
			},
			Vertex {
				position: [vertices[1].x, vertices[1].y, vertices[1].z],
				normal: normal_arr,
				uv: [1., 0f32],
			},
			Vertex {
				position: [vertices[2].x, vertices[2].y, vertices[2].z],
				normal: normal_arr,
				uv: [0.5, 1.],
			},
		]);

		self.indices.extend([base, base + 1, base + 2]);
		self.current_vertex += 3;
	}
}




const HALF: f32 = 1.;

/// Edge vertices for the marching cubes algorithm
const EDGE_VERTICES: [[Vec3; 2]; 12] = [
	[Vec3::ZERO, Vec3::new(HALF, 0f32, 0f32)], // Edge 0
	[Vec3::new(HALF, 0f32, 0f32), Vec3::new(HALF, 0f32, HALF)], // Edge 1
	[Vec3::new(HALF, 0f32, HALF), Vec3::new(0f32, 0f32, HALF)], // Edge 2
	[Vec3::new(0f32, 0f32, HALF), Vec3::ZERO], // Edge 3
	[Vec3::new(0f32, HALF, 0f32), Vec3::new(HALF, HALF, 0f32)], // Edge 4
	[Vec3::new(HALF, HALF, 0f32), Vec3::new(HALF, HALF, HALF)], // Edge 5
	[Vec3::new(HALF, HALF, HALF), Vec3::new(0f32, HALF, HALF)], // Edge 6
	[Vec3::new(0f32, HALF, HALF), Vec3::new(0f32, HALF, 0f32)], // Edge 7
	[Vec3::ZERO, Vec3::new(0f32, HALF, 0f32)], // Edge 8
	[Vec3::new(HALF, 0f32, 0f32), Vec3::new(HALF, HALF, 0f32)], // Edge 9
	[Vec3::new(HALF, 0f32, HALF), Vec3::new(HALF, HALF, HALF)], // Edge 10
	[Vec3::new(0f32, 0f32, HALF), Vec3::new(0f32, HALF, HALF)], // Edge 11
];




type Material = u16;
type DensityField = u32;

/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Block {
	None = 0,
	Simple(Material, BlockRotation), // material, rotation
	Marching(Material, DensityField), // material, density field (27 bits - 4)
}

impl Block {

	#[inline]
	pub fn is_marching(&self) -> bool {
		matches!(self, Block::Marching { .. })
	}


	/// Sets a point in the 3x3x3 density field
	#[inline]
	pub fn set_point(&mut self, x: u8, y: u8, z: u8, value: bool) {
		if let Block::Marching(_, points) = self {
			debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
			let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
			*points = (*points & !(1 << bit_pos)) | ((value as u32) << bit_pos);
		}
	}

	/// Gets a point from the 3x3x3 density field
	#[inline]
	pub fn get_point(&self, x: u8, y: u8, z: u8) -> Option<bool> {
		match self {
			Block::Marching(_, points) => {
				debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
				let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
				Some((*points & (1u32 << bit_pos)) != 0)
			}
			_ => None,
		}
	}

	pub fn get_march(&mut self) -> Option<Block> {
		match self {
			Block::Marching(_, _) => None,
			_ => Some(Self::new_march(self.material())),
		}
	}
	/// Serializes the block to a binary format
	pub fn to_binary(&self) -> Vec<u8> {
		match self {
			Block::None => vec![0],
			Block::Simple(material, rotation) => {
				let mut data = vec![1];
				data.extend_from_slice(&material.to_le_bytes());
				data.push(rotation.to_byte());
				data
			}
			Block::Marching(material, density) => {
				let mut data = vec![2];
				data.extend_from_slice(&material.to_le_bytes());
				data.extend_from_slice(&density.to_le_bytes());
				data
			}
		}
	}

	
	
	/// Deserializes the block from binary format
	pub fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() == 0 { return None; }
		let block_type = bytes.get(0)?;
		
		match block_type {
			0 => Some(Block::None),
			1 => {
				if bytes.len() < 4 { return None; }
				let material = u16::from_le_bytes([bytes[1], bytes[2]]);
				let rotation = BlockRotation::from_byte(bytes[3])?;
				Some(Block::Simple(material, rotation))
			}
			2 => {
				if bytes.len() < 7 { return None; }
				let material = u16::from_le_bytes([bytes[1], bytes[2]]);
				let density = u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
				Some(Block::Marching(material, density))
			}
			_ => None,
		}
	}
	
	/// Returns the size of the binary representation
	pub fn binary_size(&self) -> usize {
		match self {
			Block::None => 1,
			Block::Simple(_, _) => 1 + mem::size_of::<u16>() + 1,
			Block::Marching(_, _) => 1 + mem::size_of::<u16>() + mem::size_of::<u32>(),
		}
	}

}