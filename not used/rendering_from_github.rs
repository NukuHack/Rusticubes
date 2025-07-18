use std::ptr;
use std::assert;

mod constants {
	pub const CHUNK_SIZE: usize = 16;
	pub const CHUNK_SIZE_SQUARED: usize = CHUNK_SIZE * CHUNK_SIZE;
	pub const CHUNK_SIZE_CUBED: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
	pub const CHUNK_SIZE_M1: usize = CHUNK_SIZE - 1;
}

struct Voxel {
	index: u8,
	// other voxel properties...
}

struct VoxelVertex {
	// vertex data...
}

impl VoxelVertex {
	fn reset(&mut self, x: i32, y: i32, z: i32, nx: i32, ny: i32, nz: i32, u: f32, v: f32, texture_id: u8) {
		// implementation...
	}
}

struct MeshVisiter {
	visit_xn: Vec<i32>,
	visit_xp: Vec<i32>,
	visit_yn: Vec<i32>,
	visit_yp: Vec<i32>,
	visit_zn: Vec<i32>,
	visit_zp: Vec<i32>,
	comparison: i32,
}

struct TempMeshData {
	vertices: Vec<VoxelVertex>,
	write_index: usize,
}

impl TempMeshData {
	fn pre_meshing(&mut self) {
		self.write_index = 0;
	}
	
	fn write(&mut self) -> &mut VoxelVertex {
		if self.write_index >= self.vertices.len() {
			self.vertices.push(VoxelVertex::default());
		}
		let vertex = &mut self.vertices[self.write_index];
		self.write_index += 1;
		vertex
	}
}

struct Chunk {
	voxels: Vec<Voxel>,
	max_altitude: Vec<u8>,
	min_altitude: Vec<u8>,
	allocated: bool,
	fake: bool,
	// other chunk properties...
}

pub struct ChunkMesh {
	chunk: *mut Chunk,
	chunk_xn: *mut Chunk,
	chunk_xp: *mut Chunk,
	chunk_yn: *mut Chunk,
	chunk_yp: *mut Chunk,
	chunk_zn: *mut Chunk,
	chunk_zp: *mut Chunk,
	mesh_visiter: MeshVisiter,
	temp_mesh_data: TempMeshData,
	should_mesh_between_chunks: bool,
	chunk_pos_y: i32,
}

impl ChunkMesh {
	const ACCESS_STEP_Y: usize = 1;
	const ACCESS_STEP_X: usize = constants::CHUNK_SIZE;
	const ACCESS_STEP_Z: usize = constants::CHUNK_SIZE_SQUARED;

	pub unsafe fn generate_mesh(&mut self) {
		// Ensure this chunk exists
		assert!((*self.chunk).allocated, "Chunk not allocated");
		assert!(!(*self.chunk).fake, "Chunk is fake");

		// Reset mesh visiter and temp storage
		self.mesh_visiter.comparison += 1;
		self.temp_mesh_data.pre_meshing();

		// Precalculate Z voxel access
		let mut z_access = 0;

		// Get heightmap pointers
		let max_y_ptr = (*self.chunk).max_altitude.as_ptr();
		let min_y_ptr = (*self.chunk).min_altitude.as_ptr();

		for k in 0..constants::CHUNK_SIZE {
			// Precalculate X voxel access
			let mut x_access = 0;

			for i in 0..constants::CHUNK_SIZE {
				// Get the min and max bounds for this column
				let j = *min_y_ptr.add(i + k * constants::CHUNK_SIZE) as usize;
				let max_j = *max_y_ptr.add(i + k * constants::CHUNK_SIZE) as usize;

				// Precalculate voxel access
				let mut access = z_access + x_access + j;
				let mut voxel_ptr = (*self.chunk).voxels.as_ptr().add(access);

				// Mesh from the bottom to the top of this column
				for current_j in j..=max_j {
					if (*voxel_ptr).index > 0 {
						self.create_runs(voxel_ptr, i, current_j, k, access, x_access, z_access);
					}
					access += Self::ACCESS_STEP_Y;
					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
				}

				// Update voxel access
				x_access += Self::ACCESS_STEP_X;
			}

			// Update voxel access
			z_access += Self::ACCESS_STEP_Z;
		}
	}

	unsafe fn create_runs(
		&mut self,
		voxel: *const Voxel,
		i: usize,
		j: usize,
		k: usize,
		access: usize,
		x_access: usize,
		z_access: usize,
	) {
		// Check if we're on the edge of this chunk
		let min_x = i == 0;
		let max_x = i == constants::CHUNK_SIZE_M1;
		let min_z = k == 0;
		let max_z = k == constants::CHUNK_SIZE_M1;
		let min_y = j == 0;
		let max_y = j == constants::CHUNK_SIZE_M1;

		// Precalculate mesh visiters for each face
		let visit_xn = self.mesh_visiter.visit_xn.as_mut_ptr().add(access);
		let visit_xp = self.mesh_visiter.visit_xp.as_mut_ptr().add(access);
		let visit_yn = self.mesh_visiter.visit_yn.as_mut_ptr().add(access);
		let visit_yp = self.mesh_visiter.visit_yp.as_mut_ptr().add(access);
		let visit_zn = self.mesh_visiter.visit_zn.as_mut_ptr().add(access);
		let visit_zp = self.mesh_visiter.visit_zp.as_mut_ptr().add(access);

		// Precalculate common values
		let data = (*self.chunk).voxels.as_ptr();
		let index = (*voxel).index;
		let comparison = self.mesh_visiter.comparison;
		let texture_id = index;
		let i1 = i + 1;
		let j1 = j + 1;

		let mut end_a;
		let mut length_b;

		// Left (X-)
		if *visit_xn != comparison && self.draw_face_xn(j, voxel, min_x, z_access, index) {
			let original_xn = visit_xn;

			// Remember we've meshed this face
			*visit_xn = comparison;
			let mut visit_xn = visit_xn.add(Self::ACCESS_STEP_Y);

			// Combine faces upwards along the Y axis
			let mut voxel_ptr = data.add(access + Self::ACCESS_STEP_Y);
			let mut y_access = j1;

			end_a = j1;
			while end_a < constants::CHUNK_SIZE {
				if (*voxel_ptr).index != index
					|| !self.draw_face_xn(y_access, voxel_ptr, min_x, z_access, index)
					|| *visit_xn == comparison
				{
					break;
				}

				// Step upwards
				voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
				y_access += 1;

				// Remember we've meshed this face
				*visit_xn = comparison;
				visit_xn = visit_xn.add(Self::ACCESS_STEP_Y);
				end_a += 1;
			}

			// Calculate how many voxels we combined along the Y axis
			let length_a = end_a - j1 + 1;

			// Combine faces along the Z axis
			length_b = 1;

			let max_length_b = constants::CHUNK_SIZE - k;
			let net_z_access = z_access;

			for g in 1..max_length_b {
				// Go back to where we started, then move g units along the Z axis
				voxel_ptr = data.add(access).add(Self::ACCESS_STEP_Z * g);
				y_access = j;

				// Check if the entire row next to us is also the same index and not covered by another block
				let mut adjacent_row_is_identical = true;

				for test_a in j..end_a {
					// No need to check the meshVisiter here as we're combining on this axis for the first time
					if (*voxel_ptr).index != index
						|| !self.draw_face_xn(y_access, voxel_ptr, min_x, net_z_access, index)
					{
						adjacent_row_is_identical = false;
						break;
					}

					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
					y_access += 1;
				}

				if !adjacent_row_is_identical {
					break;
				}

				// We found a whole row that's valid!
				length_b += 1;

				// Remember we've meshed these faces
				let mut temp_xn = original_xn.add(Self::ACCESS_STEP_Z * g);

				for _ in 0..length_a {
					*temp_xn = comparison;
					temp_xn = temp_xn.add(Self::ACCESS_STEP_Y);
				}
			}

			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				k as i32,
				-1,
				0,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				(k + length_b) as i32,
				-1,
				0,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				k as i32,
				-1,
				0,
				0,
				1.0,
				0.0,
				texture_id,
			);

			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				k as i32,
				-1,
				0,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				(k + length_b) as i32,
				-1,
				0,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				(k + length_b) as i32,
				-1,
				0,
				0,
				1.0,
				0.0,
				texture_id,
			);
		}

		// Right (X+)
		if *visit_xp != comparison && self.draw_face_xp(j, voxel, max_x, z_access, index) {
			let original_xp = visit_xp;

			// Remember we've meshed this face
			*visit_xp = comparison;
			let mut visit_xp = visit_xp.add(Self::ACCESS_STEP_Y);

			// Combine faces along the Y axis
			let mut voxel_ptr = data.add(access + Self::ACCESS_STEP_Y);
			let mut y_access = j1;

			end_a = j1;
			while end_a < constants::CHUNK_SIZE {
				if (*voxel_ptr).index != index
					|| !self.draw_face_xp(y_access, voxel_ptr, max_x, z_access, index)
					|| *visit_xp == comparison
				{
					break;
				}

				voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
				y_access += 1;

				// Remember we've meshed this face
				*visit_xp = comparison;
				visit_xp = visit_xp.add(Self::ACCESS_STEP_Y);
				end_a += 1;
			}

			// Calculate how many voxels we combined along the Y axis
			let length_a = end_a - j1 + 1;

			// Combine faces along the Z axis
			length_b = 1;

			let max_length_b = constants::CHUNK_SIZE - k;
			let net_z_access = z_access;

			for g in 1..max_length_b {
				// Go back to where we started, then move g units on the Z axis
				voxel_ptr = data.add(access).add(Self::ACCESS_STEP_Z * g);
				y_access = j;

				// Check if the entire row next to us is also the same index and not covered by another block
				let mut adjacent_row_is_identical = true;

				for test_a in j..end_a {
					if (*voxel_ptr).index != index
						|| !self.draw_face_xp(y_access, voxel_ptr, max_x, net_z_access, index)
					{
						adjacent_row_is_identical = false;
						break;
					}

					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
					y_access += 1;
				}

				if !adjacent_row_is_identical {
					break;
				}

				// We found a whole row that's valid!
				length_b += 1;

				// Remember we've meshed these faces
				let mut temp_xp = original_xp.add(Self::ACCESS_STEP_Z * g);

				for _ in 0..length_a {
					*temp_xp = comparison;
					temp_xp = temp_xp.add(Self::ACCESS_STEP_Y);
				}
			}

			self.temp_mesh_data.write().reset(
				(i + 1) as i32,
				j as i32,
				k as i32,
				1,
				0,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + 1) as i32,
				(j + length_a) as i32,
				k as i32,
				1,
				0,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + 1) as i32,
				j as i32,
				(k + length_b) as i32,
				1,
				0,
				0,
				1.0,
				0.0,
				texture_id,
			);

			self.temp_mesh_data.write().reset(
				(i + 1) as i32,
				j as i32,
				(k + length_b) as i32,
				1,
				0,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + 1) as i32,
				(j + length_a) as i32,
				k as i32,
				1,
				0,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + 1) as i32,
				(j + length_a) as i32,
				(k + length_b) as i32,
				1,
				0,
				0,
				1.0,
				0.0,
				texture_id,
			);
		}

		// Back (Z-)
		if *visit_zn != comparison && self.draw_face_zn(j, voxel, min_z, x_access, index) {
			let original_zn = visit_zn;

			// Remember we've meshed this face
			*visit_zn = comparison;
			let mut visit_zn = visit_zn.add(Self::ACCESS_STEP_Y);

			// Combine faces along the Y axis
			let mut voxel_ptr = data.add(access + Self::ACCESS_STEP_Y);
			let mut y_access = j1;

			end_a = j1;
			while end_a < constants::CHUNK_SIZE {
				if (*voxel_ptr).index != index
					|| !self.draw_face_zn(y_access, voxel_ptr, min_z, x_access, index)
					|| *visit_zn == comparison
				{
					break;
				}

				voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
				y_access += 1;

				// Remember we've meshed this face
				*visit_zn = comparison;
				visit_zn = visit_zn.add(Self::ACCESS_STEP_Y);
				end_a += 1;
			}

			// Calculate how many voxels we combined along the Y axis
			let length_a = end_a - j1 + 1;

			// Combine faces along the X axis
			length_b = 1;

			let max_length_b = constants::CHUNK_SIZE - i;

			for g in 1..max_length_b {
				// Go back to where we started, then move g units on the X axis
				voxel_ptr = data.add(access).add(Self::ACCESS_STEP_X * g);
				y_access = j;

				// Check if the entire row next to us is also the same index and not covered by another block
				let mut adjacent_row_is_identical = true;

				for test_a in j..end_a {
					if (*voxel_ptr).index != index
						|| !self.draw_face_zn(y_access, voxel_ptr, min_z, x_access, index)
					{
						adjacent_row_is_identical = false;
						break;
					}

					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
					y_access += 1;
				}

				if !adjacent_row_is_identical {
					break;
				}

				// We found a whole row that's valid!
				length_b += 1;

				// Remember we've meshed these faces
				let mut temp_zn = original_zn.add(Self::ACCESS_STEP_X * g);

				for _ in 0..length_a {
					*temp_zn = comparison;
					temp_zn = temp_zn.add(Self::ACCESS_STEP_Y);
				}
			}

			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				k as i32,
				0,
				0,
				-1,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				k as i32,
				0,
				0,
				-1,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_b) as i32,
				j as i32,
				k as i32,
				0,
				0,
				-1,
				1.0,
				0.0,
				texture_id,
			);

			self.temp_mesh_data.write().reset(
				(i + length_b) as i32,
				j as i32,
				k as i32,
				0,
				0,
				-1,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				k as i32,
				0,
				0,
				-1,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_b) as i32,
				(j + length_a) as i32,
				k as i32,
				0,
				0,
				-1,
				1.0,
				0.0,
				texture_id,
			);
		}

		// Front (Z+)
		if *visit_zp != comparison && self.draw_face_zp(j, voxel, max_z, x_access, index) {
			let original_zp = visit_zp;

			// Remember we've meshed this face
			*visit_zp = comparison;
			let mut visit_zp = visit_zp.add(Self::ACCESS_STEP_Y);

			// Combine faces along the Y axis
			let mut voxel_ptr = data.add(access + Self::ACCESS_STEP_Y);
			let mut y_access = j1;

			end_a = j1;
			while end_a < constants::CHUNK_SIZE {
				if (*voxel_ptr).index != index
					|| !self.draw_face_zp(y_access, voxel_ptr, max_z, x_access, index)
					|| *visit_zp == comparison
				{
					break;
				}

				voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
				y_access += 1;

				// Remember we've meshed this face
				*visit_zp = comparison;
				visit_zp = visit_zp.add(Self::ACCESS_STEP_Y);
				end_a += 1;
			}

			// Calculate how many voxels we combined along the Y axis
			let length_a = end_a - j1 + 1;

			// Combine faces along the X axis
			length_b = 1;

			let max_length_b = constants::CHUNK_SIZE - i;

			for g in 1..max_length_b {
				// Go back to where we started, then move g units on the X axis
				voxel_ptr = data.add(access).add(Self::ACCESS_STEP_X * g);
				y_access = j;

				// Check if the entire row next to us is also the same index and not covered by another block
				let mut adjacent_row_is_identical = true;

				for test_a in j..end_a {
					if (*voxel_ptr).index != index
						|| !self.draw_face_zp(y_access, voxel_ptr, max_z, x_access, index)
					{
						adjacent_row_is_identical = false;
						break;
					}

					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_Y);
					y_access += 1;
				}

				if !adjacent_row_is_identical {
					break;
				}

				// We found a whole row that's valid!
				length_b += 1;

				// Remember we've meshed these faces
				let mut temp_zp = original_zp.add(Self::ACCESS_STEP_X * g);

				for _ in 0..length_a {
					*temp_zp = comparison;
					temp_zp = temp_zp.add(Self::ACCESS_STEP_Y);
				}
			}

			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				(k + 1) as i32,
				0,
				0,
				1,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_b) as i32,
				j as i32,
				(k + 1) as i32,
				0,
				0,
				1,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				(k + 1) as i32,
				0,
				0,
				1,
				1.0,
				0.0,
				texture_id,
			);

			self.temp_mesh_data.write().reset(
				i as i32,
				(j + length_a) as i32,
				(k + 1) as i32,
				0,
				0,
				1,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_b) as i32,
				j as i32,
				(k + 1) as i32,
				0,
				0,
				1,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_b) as i32,
				(j + length_a) as i32,
				(k + 1) as i32,
				0,
				0,
				1,
				1.0,
				0.0,
				texture_id,
			);
		}

		// Bottom (Y-)
		if *visit_yn != comparison && self.draw_face_yn(voxel, min_y, x_access, z_access, index) {
			let original_yn = visit_yn;

			// Remember we've meshed this face
			*visit_yn = comparison;
			let mut visit_yn = visit_yn.add(Self::ACCESS_STEP_X);

			// Combine faces along the X axis
			let mut voxel_ptr = data.add(access + Self::ACCESS_STEP_X);
			let mut net_x_access = x_access + Self::ACCESS_STEP_X;

			end_a = i1;
			while end_a < constants::CHUNK_SIZE {
				if (*voxel_ptr).index != index
					|| !self.draw_face_yn(voxel_ptr, min_y, net_x_access, z_access, index)
					|| *visit_yn == comparison
				{
					break;
				}

				// Remember we've meshed this face
				*visit_yn = comparison;
				visit_yn = visit_yn.add(Self::ACCESS_STEP_X);

				// Move 1 unit on the X axis
				voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_X);
				net_x_access += Self::ACCESS_STEP_X;
				end_a += 1;
			}

			// Calculate how many voxels we combined along the X axis
			let length_a = end_a - i1 + 1;

			// Combine faces along the Z axis
			length_b = 1;

			let max_length_b = constants::CHUNK_SIZE - k;

			for g in 1..max_length_b {
				// Go back to where we started, then move g units on the Z axis
				voxel_ptr = data.add(access).add(Self::ACCESS_STEP_Z * g);
				net_x_access = x_access;

				// Check if the entire row next to us is also the same index and not covered by another block
				let mut adjacent_row_is_identical = true;

				for test_a in i..end_a {
					if (*voxel_ptr).index != index
						|| !self.draw_face_yn(voxel_ptr, min_y, net_x_access, z_access, index)
					{
						adjacent_row_is_identical = false;
						break;
					}

					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_X);
					net_x_access += Self::ACCESS_STEP_X;
				}

				if !adjacent_row_is_identical {
					break;
				}

				// We found a whole row that's valid!
				length_b += 1;

				// Remember we've meshed these faces
				let mut temp_yn = original_yn.add(Self::ACCESS_STEP_Z * g);

				for _ in 0..length_a {
					*temp_yn = comparison;
					temp_yn = temp_yn.add(Self::ACCESS_STEP_X);
				}
			}

			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				k as i32,
				0,
				-1,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_a) as i32,
				j as i32,
				k as i32,
				0,
				-1,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				(k + length_b) as i32,
				0,
				-1,
				0,
				1.0,
				0.0,
				texture_id,
			);

			self.temp_mesh_data.write().reset(
				i as i32,
				j as i32,
				(k + length_b) as i32,
				0,
				-1,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_a) as i32,
				j as i32,
				k as i32,
				0,
				-1,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_a) as i32,
				j as i32,
				(k + length_b) as i32,
				0,
				-1,
				0,
				1.0,
				0.0,
				texture_id,
			);
		}

		// Top (Y+)
		if *visit_yp != comparison && self.draw_face_yp(voxel, max_y, x_access, z_access, index) {
			let original_yp = visit_yp;

			// Remember we've meshed this face
			*visit_yp = comparison;
			let mut visit_yp = visit_yp.add(Self::ACCESS_STEP_X);

			// Combine faces along the X axis
			let mut voxel_ptr = data.add(access + Self::ACCESS_STEP_X);
			let mut net_x_access = x_access + Self::ACCESS_STEP_X;

			end_a = i1;
			while end_a < constants::CHUNK_SIZE {
				if (*voxel_ptr).index != index
					|| !self.draw_face_yp(voxel_ptr, max_y, net_x_access, z_access, index)
					|| *visit_yp == comparison
				{
					break;
				}

				// Remember we've meshed this face
				*visit_yp = comparison;
				visit_yp = visit_yp.add(Self::ACCESS_STEP_X);

				// Move 1 unit on the X axis
				voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_X);
				net_x_access += Self::ACCESS_STEP_X;
				end_a += 1;
			}

			// Calculate how many voxels we combined along the X axis
			let length_a = end_a - i1 + 1;

			// Combine faces along the Z axis
			length_b = 1;

			let max_length_b = constants::CHUNK_SIZE - k;

			for g in 1..max_length_b {
				// Go back to where we started, then move g units on the Z axis
				voxel_ptr = data.add(access).add(Self::ACCESS_STEP_Z * g);
				net_x_access = x_access;

				// Check if the entire row next to us is also the same index and not covered by another block
				let mut adjacent_row_is_identical = true;

				for test_a in i..end_a {
					if (*voxel_ptr).index != index
						|| !self.draw_face_yp(voxel_ptr, max_y, net_x_access, z_access, index)
					{
						adjacent_row_is_identical = false;
						break;
					}

					voxel_ptr = voxel_ptr.add(Self::ACCESS_STEP_X);
					net_x_access += Self::ACCESS_STEP_X;
				}

				if !adjacent_row_is_identical {
					break;
				}

				// We found a whole row that's valid!
				length_b += 1;

				// Remember we've meshed these faces
				let mut temp_yp = original_yp.add(Self::ACCESS_STEP_Z * g);

				for _ in 0..length_a {
					*temp_yp = comparison;
					temp_yp = temp_yp.add(Self::ACCESS_STEP_X);
				}
			}

			self.temp_mesh_data.write().reset(
				i as i32,
				(j + 1) as i32,
				k as i32,
				0,
				1,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + 1) as i32,
				(k + length_b) as i32,
				0,
				1,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_a) as i32,
				(j + 1) as i32,
				k as i32,
				0,
				1,
				0,
				1.0,
				0.0,
				texture_id,
			);

			self.temp_mesh_data.write().reset(
				(i + length_a) as i32,
				(j + 1) as i32,
				k as i32,
				0,
				1,
				0,
				0.0,
				0.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				i as i32,
				(j + 1) as i32,
				(k + length_b) as i32,
				0,
				1,
				0,
				0.0,
				1.0,
				texture_id,
			);
			self.temp_mesh_data.write().reset(
				(i + length_a) as i32,
				(j + 1) as i32,
				(k + length_b) as i32,
				0,
				1,
				0,
				1.0,
				0.0,
				texture_id,
			);
		}
	}

	unsafe fn draw_face_common(&self, next_ptr: *const Voxel, index: u8) -> bool {
		if (*next_ptr).index == 0 {
			return true;
		}
		false
	}

	unsafe fn draw_face_xn(
		&self,
		j: usize,
		b_ptr: *const Voxel,
		min: bool,
		k_cs2: usize,
		index: u8,
	) -> bool {
		// If it is outside this chunk, get the voxel from the neighbouring chunk
		if min {
			if self.should_mesh_between_chunks {
				return true;
			}

			if !(*self.chunk_xn).allocated {
				return true;
			}

			return self.draw_face_common(
				(*self.chunk_xn)
					.voxels
					.as_ptr()
					.add((constants::CHUNK_SIZE - 1) * constants::CHUNK_SIZE + j + k_cs2),
				index,
			);
		}

		self.draw_face_common(b_ptr.sub(constants::CHUNK_SIZE), index)
	}

	unsafe fn draw_face_xp(
		&self,
		j: usize,
		b_ptr: *const Voxel,
		max: bool,
		k_cs2: usize,
		index: u8,
	) -> bool {
		if max {
			if self.should_mesh_between_chunks {
				return true;
			}

			// If no chunk next to us, render
			if !(*self.chunk_xp).allocated {
				return true;
			}

			return self.draw_face_common(
				(*self.chunk_xp).voxels.as_ptr().add(j + k_cs2),
				index,
			);
		}

		self.draw_face_common(b_ptr.add(constants::CHUNK_SIZE), index)
	}

	unsafe fn draw_face_yn(
		&self,
		b_ptr: *const Voxel,
		min: bool,
		i_cs: usize,
		k_cs2: usize,
		index: u8,
	) -> bool {
		if min {
			if self.chunk_pos_y == 0 {
				// Handle special case for bottom of world
			}
			if self.should_mesh_between_chunks {
				return true;
			}

			// If there's no chunk below us, render the face
			if !(*self.chunk_yn).allocated {
				return true;
			}

			return self.draw_face_common(
				(*self.chunk_yn)
					.voxels
					.as_ptr()
					.add(i_cs + (constants::CHUNK_SIZE - 1) + k_cs2),
				index,
			);
		}

		self.draw_face_common(b_ptr.sub(Self::ACCESS_STEP_Y), index)
	}

	unsafe fn draw_face_yp(
		&self,
		voxel_ptr: *const Voxel,
		max: bool,
		x_access: usize,
		z_access: usize,
		index: u8,
	) -> bool {
		if max {
			if self.should_mesh_between_chunks {
				return true;
			}

			// If there's no chunk above us, render
			if !(*self.chunk_yp).allocated {
				return true;
			}

			// Check if there's a block in the bottom layer of the chunk above us
			return self.draw_face_common(
				(*self.chunk_yp).voxels.as_ptr().add(x_access + z_access),
				index,
			);
		}

		// Check if the block above us is the same index
		self.draw_face_common(voxel_ptr.add(Self::ACCESS_STEP_Y), index)
	}

	unsafe fn draw_face_zn(
		&self,
		j: usize,
		b_ptr: *const Voxel,
		min: bool,
		i_cs: usize,
		index: u8,
	) -> bool {
		if min {
			if self.should_mesh_between_chunks {
				return true;
			}

			// if there's no chunk next to us, render
			if !(*self.chunk_zn).allocated {
				return true;
			}

			return self.draw_face_common(
				(*self.chunk_zn)
					.voxels
					.as_ptr()
					.add(i_cs + j + (constants::CHUNK_SIZE - 1) * constants::CHUNK_SIZE_SQUARED),
				index,
			);
		}

		self.draw_face_common(b_ptr.sub(constants::CHUNK_SIZE_SQUARED), index)
	}

	unsafe fn draw_face_zp(
		&self,
		j: usize,
		b_ptr: *const Voxel,
		max: bool,
		i_cs: usize,
		index: u8,
	) -> bool {
		if max {
			if self.should_mesh_between_chunks {
				return true;
			}

			// If no chunk next to us, render
			if !(*self.chunk_zp).allocated {
				return true;
			}

			return self.draw_face_common(
				(*self.chunk_zp).voxels.as_ptr().add(i_cs + j),
				index,
			);
		}

		self.draw_face_common(b_ptr.add(constants::CHUNK_SIZE_SQUARED), index)
	}
}