use super::camera::*;
use glam::Vec3;
use winit::event::*;
use winit::keyboard::KeyCode as Key;

pub struct Player {
    pub position: Vec3,
    pub controller: PlayerController,
}

impl Player {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            position: Vec3::ZERO,
            controller: PlayerController::new(config),
        }
    }
}

pub struct PlayerController {
    pub movement: MovementInputs,
    pub rotation: Vec3,
    pub config: CameraConfig,
    pub scroll: f32,
    pub velocity: Vec3,
    pub target_rotation: Vec3,
    pub current_rotation: Vec3,
}

#[derive(Debug, Default)]
pub struct MovementInputs {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub run: bool,
}

impl PlayerController {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            movement: MovementInputs::default(),
            rotation: Vec3::ZERO,
            scroll: 0.0,
            config,
            velocity: Vec3::ZERO,
            target_rotation: Vec3::new(config.yaw, config.pitch, 0.0),
            current_rotation: Vec3::new(config.yaw, config.pitch, 0.0),
        }
    }

    pub fn process_keyboard(&mut self, key: &Key, state: &ElementState) -> bool {
        let is_pressed = *state == ElementState::Pressed;
        match key {
            Key::KeyW | Key::ArrowUp => self.movement.forward = is_pressed,
            Key::KeyS | Key::ArrowDown => self.movement.backward = is_pressed,
            Key::KeyA | Key::ArrowLeft => self.movement.left = is_pressed,
            Key::KeyD | Key::ArrowRight => self.movement.right = is_pressed,
            Key::Space => self.movement.up = is_pressed,
            Key::ShiftLeft => self.movement.run = is_pressed,
            Key::ControlLeft => self.movement.down = is_pressed,
            _ => return false,
        }
        true
    }

    pub fn reset_keyboard(&mut self) {
        self.movement = MovementInputs::default();
    }

    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.rotation.x = -delta_x;
        self.rotation.y = -delta_y;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 0.5,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
        };
    }

    pub fn update(
        &mut self,
        camera: &mut Camera,
        projection: &mut Projection,
        delta_time: f32,
    ) -> Vec3 {
        let dt = delta_time.min(0.1);

        // Update rotation with smoothing
        self.target_rotation.x += self.rotation.x * self.config.sensitivity * 0.05;
        self.target_rotation.y += self.rotation.y * self.config.sensitivity * 0.05;
        self.target_rotation.z += self.rotation.z * self.config.sensitivity * 0.05;
        self.target_rotation.y = self
            .target_rotation
            .y
            .clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        let smooth_factor = 1.0 - (-self.config.smoothness * dt).exp();
        self.current_rotation = self
            .current_rotation
            .lerp(self.target_rotation, smooth_factor);

        camera.set_yaw(self.current_rotation.x);
        camera.set_pitch(self.current_rotation.y);

        self.rotation.x = 0.0;
        self.rotation.y = 0.0;

        // Movement calculations
        let run_multiplier = if self.movement.run {
            self.config.run_multiplier
        } else {
            1.0
        };
        let speed = self.config.speed * run_multiplier;

        let forward_amount = (self.movement.forward as i8 - self.movement.backward as i8) as f32;
        let right_amount = (self.movement.right as i8 - self.movement.left as i8) as f32;
        let up_amount = (self.movement.up as i8 - self.movement.down as i8) as f32;

        let target_velocity = (camera.forward() * forward_amount
            + camera.right() * right_amount
            + camera.up() * up_amount)
            * speed;

        let acceleration = if target_velocity.length_squared() > 0.0 {
            10.0
        } else {
            20.0
        };
        self.velocity = self.velocity.lerp(target_velocity, acceleration * dt);

        // Handle zoom
        if self.scroll.abs() > f32::EPSILON {
            let delta = self.scroll * self.config.sensitivity;
            projection.set_fovy((projection.fovy - delta).clamp(0.001, std::f32::consts::PI));
            self.scroll = 0.0;
        }

        self.velocity * dt
    }

    pub fn reset(&mut self, camera: &mut Camera) {
        self.movement = MovementInputs::default();
        self.velocity = Vec3::ZERO;
        self.target_rotation = Vec3::new(camera.yaw(), camera.pitch(), 0.0);
        self.current_rotation = self.target_rotation;
    }
}
