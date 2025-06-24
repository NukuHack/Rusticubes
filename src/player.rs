use super::camera::*;
use glam::Vec3;
use winit::event::*;
use winit::keyboard::KeyCode as Key;

pub struct Player {
    pub position: Vec3,
    pub config: CameraConfig,
    pub controller: PlayerController,
}

impl Player {
    #[inline]
    pub fn new(config: CameraConfig) -> Self {
        Self {
            position: Vec3::ZERO,
            config,
            controller: PlayerController::new(config),
        }
    }
    #[inline]
    pub fn update(
        &mut self,
        camera: &mut Camera,
        projection: &mut Projection,
        delta_time: f32,
    ) -> Vec3 {
        let dt = delta_time.min(0.1);

        // Update rotation with smoothing
        self.controller.target_rotation.x +=
            self.controller.rotation.x * self.config.sensitivity * 0.05;
        self.controller.target_rotation.y +=
            self.controller.rotation.y * self.config.sensitivity * 0.05;
        self.controller.target_rotation.z +=
            self.controller.rotation.z * self.config.sensitivity * 0.05;
        self.controller.target_rotation.y = self
            .controller
            .target_rotation
            .y
            .clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        let smooth_factor = 1.0 - (-self.config.smoothness * dt).exp();
        self.controller.current_rotation = self
            .controller
            .current_rotation
            .lerp(self.controller.target_rotation, smooth_factor);

        camera.rotation = self.controller.current_rotation;

        self.controller.rotation = Vec3::ZERO;

        // Movement calculations
        let run_multiplier = if self.controller.movement.run {
            self.config.run_multiplier
        } else {
            1.0
        };
        let speed = self.config.speed * run_multiplier;

        let forward_amount = (self.controller.movement.forward as i8
            - self.controller.movement.backward as i8) as f32;
        let right_amount =
            (self.controller.movement.right as i8 - self.controller.movement.left as i8) as f32;
        let up_amount =
            (self.controller.movement.up as i8 - self.controller.movement.down as i8) as f32;

        let target_velocity = (camera.forward() * forward_amount
            + camera.right() * right_amount
            + camera.up() * up_amount)
            * speed;

        let acceleration = if target_velocity.length_squared() > 0.0 {
            10.0
        } else {
            20.0
        };
        self.controller.velocity = self
            .controller
            .velocity
            .lerp(target_velocity, acceleration * dt);

        // Handle zoom
        if self.controller.scroll.abs() > f32::EPSILON {
            let delta = self.controller.scroll * self.config.sensitivity;
            projection.set_fovy((projection.fovy - delta).clamp(0.001, std::f32::consts::PI));
            self.controller.scroll = 0.0;
        }

        self.controller.velocity * dt
    }
}

pub struct PlayerController {
    pub movement: MovementInputs,
    pub rotation: Vec3,
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

#[allow(dead_code)]
impl PlayerController {
    #[inline]
    pub fn new(config: CameraConfig) -> Self {
        Self {
            movement: MovementInputs::default(),
            rotation: Vec3::ZERO,
            scroll: 0.0,
            velocity: Vec3::ZERO,
            target_rotation: config.rotation,
            current_rotation: config.rotation,
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
    #[inline]
    pub fn reset_keyboard(&mut self) {
        self.movement = MovementInputs::default();
    }
    #[inline]
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.rotation.x = -delta_x;
        self.rotation.y = -delta_y;
    }
    #[inline]
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 0.5,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
        };
    }
    #[inline]
    pub fn reset(&mut self, camera: &mut Camera) {
        self.movement = MovementInputs::default();
        self.velocity = Vec3::ZERO;
        self.target_rotation = camera.rotation;
        self.current_rotation = self.target_rotation;
    }
}
