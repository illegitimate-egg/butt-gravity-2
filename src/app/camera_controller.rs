use core::f64;
use std::sync::Arc;

use glam::{DQuat, DVec3};
use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
    window::{CursorGrabMode, Window},
};

use crate::renderer::camera::Camera;

pub struct CameraController {
    speed: f32,
    sensitivity: f64,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_right_pressed: bool,
    is_left_pressed: bool,
    is_mouse_button_pressed: bool,
    delta: (f64, f64),
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f64) -> Self {
        Self {
            speed,
            sensitivity,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_right_pressed: false,
            is_left_pressed: false,
            is_mouse_button_pressed: false,
            delta: (0.0, 0.0),
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn handle_click(
        &mut self,
        element_state: ElementState,
        button: MouseButton,
        window: Arc<Window>,
    ) {
        if button == MouseButton::Left {
            self.is_mouse_button_pressed = element_state == ElementState::Pressed;

            if self.is_mouse_button_pressed {
                let _ = window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
                window.set_cursor_visible(false);
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
            }
        }
    }

    pub fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        if self.is_mouse_button_pressed {
            self.delta.0 += delta.0;
            self.delta.1 += delta.1;
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        if self.is_forward_pressed {
            camera.position += camera.forward() * self.speed as f64;
        }
        if self.is_backward_pressed {
            camera.position -= camera.forward() * self.speed as f64;
        }
        if self.is_right_pressed {
            camera.position += camera.right() * self.speed as f64;
        }
        if self.is_left_pressed {
            camera.position -= camera.right() * self.speed as f64;
        }

        let yaw_angle = self.delta.0 * self.sensitivity;
        let pitch_angle = self.delta.1 * self.sensitivity;

        let q_yaw = DQuat::from_axis_angle(DVec3::Y, -yaw_angle);
        let q_pitch = DQuat::from_axis_angle(DVec3::X, -pitch_angle);

        camera.orientation = (q_yaw * camera.orientation * q_pitch).normalize();

        self.delta = (0.0, 0.0);
    }
}
