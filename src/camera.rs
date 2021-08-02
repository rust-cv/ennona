use std::time::Duration;

use nalgebra::{IsometryMatrix3, Matrix4, Rotation3, Vector3, Vector4};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta, VirtualKeyCode},
};

pub struct Camera {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub view_matrix: IsometryMatrix3<f32>,
    // pitch: f32,
    // yaw: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let perspective = Matrix4::from_diagonal(&Vector4::new(-1., -1., 1., 1.))
            * Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar);

        perspective * self.view_matrix.to_matrix()
    }
}

pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub is_up_pressed: bool,
    pub is_down_pressed: bool,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_counter_clock_pressed: bool,
    pub is_clock_pressed: bool,
    pub mouse_captured: bool,
    pub rotate_horizontal: f32,
    pub rotate_vertical: f32,
    pub scroll: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_clock_pressed: false,
            is_counter_clock_pressed: false,
            mouse_captured: false,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
        }
    }

    pub fn set_speed(&mut self, new_speed: f32) {
        self.speed = new_speed;
    }

    pub fn set_sensitivity(&mut self, new_sensitivity: f32) {
        self.sensitivity = new_sensitivity;
    }

    pub fn process_keyboard(&mut self, keycode: &VirtualKeyCode, state: ElementState) -> bool {
        let is_pressed = state == ElementState::Pressed;
        match keycode {
            VirtualKeyCode::Space => {
                self.is_up_pressed = is_pressed;
                true
            }
            VirtualKeyCode::LShift => {
                self.is_down_pressed = is_pressed;
                true
            }
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.is_forward_pressed = is_pressed;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.is_left_pressed = is_pressed;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.is_backward_pressed = is_pressed;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.is_right_pressed = is_pressed;
                true
            }

            VirtualKeyCode::Q => {
                self.is_counter_clock_pressed = is_pressed;
                true
            }
            VirtualKeyCode::E => {
                self.is_clock_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();
        if self.rotate_horizontal != 0. {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(
                    0.,
                    -self.rotate_horizontal * self.sensitivity * dt,
                    0.,
                ));
        }

        if self.rotate_vertical != 0. {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(
                    -self.rotate_vertical * self.sensitivity * dt,
                    0.,
                    0.,
                ));
        }

        if self.is_forward_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(self.speed * Vector3::z()).into());
        }
        if self.is_backward_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(self.speed * -Vector3::z()).into());
        }

        if self.is_right_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(self.speed * Vector3::x()).into());
        }
        if self.is_left_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(self.speed * -Vector3::x()).into());
        }

        if self.is_up_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(self.speed * Vector3::y()).into());
        }
        if self.is_down_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(self.speed * -Vector3::y()).into());
        }
        if self.is_clock_pressed {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(0., 0., -self.speed * 0.05));
        }
        if self.is_counter_clock_pressed {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(0., 0., self.speed * 0.05));
        }
    }
}
