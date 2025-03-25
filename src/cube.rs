

pub struct Cube {
    pub position: [f32; 3],
    pub material: u32,
    pub points: [[bool; 3];8],
    pub rotation: [f32; 3],
}

impl Cube {
    fn nice() -> bool {
        true
    }

    fn default() -> Self {
        Self {
            position:[0f32,0f32,0f32],
            material: 1,
            points: Self::point_def(),
            rotation:[0f32,0f32,0f32],
        }
    }
    fn point_def() -> [[bool; 3];8] {
        let array:[[bool; 3];8] = [
            [true,true,true],
            [false,true,true],
            [true,false,true],
            [false,false,true],
            [true,true,false],
            [false,true,false],
            [true,false,false],
            [false,false,false],
        ];
        array
    }

    pub fn get_mesh() {

    }
}