use std::time::Duration;

use nalgebra::{Translation3, Vector2, Vector3};
use winit::event::{ElementState, VirtualKeyCode};

use super::Camera;

pub struct FlyCamController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    speed: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    sensitivity: f32,
}

impl FlyCamController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount * 0.75;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount * 0.75;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let frame = camera.observer_frame();
        let forward = frame * Vector3::z();
        let right = frame * Vector3::x();
        let up = frame * Vector3::y();

        let forward_disp = forward * (self.amount_forward - self.amount_backward);
        let right_disp = right * -(self.amount_right - self.amount_left);
        let up_disp = up * (self.amount_up - self.amount_down);

        camera.translate_mut(&Translation3::from(
            (forward_disp + right_disp + up_disp) * self.speed * dt,
        ));

        camera.rotate_mut(&Vector2::new(
            self.rotate_horizontal * self.sensitivity * dt,
            self.rotate_vertical * self.sensitivity * dt * 0.75,
        ));

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
    }
}
