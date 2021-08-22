use std::time::Duration;

use nalgebra::{IsometryMatrix3, Matrix4, Point3, Rotation3, Vector3, Vector4};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta, VirtualKeyCode},
    window::Window,
};

pub struct Camera {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub view_matrix: IsometryMatrix3<f32>,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let perspective = Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar)
            * Matrix4::from_diagonal(&Vector4::new(1.0, -1.0, -1.0, 1.0));

        perspective * self.view_matrix.to_matrix()
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.aspect = new_size.width as f32 / new_size.height as f32;
    }

    /// Sets the camera to be facing `target` while also changing the near and far clip planes
    /// based on the set distance.
    pub fn set_camera_facing(&mut self, target: Point3<f32>, distance: f32) {
        self.zfar = 100. * distance;
        self.znear = distance / 100.;
        self.view_matrix = IsometryMatrix3::translation(0.0, 0.0, distance)
            * IsometryMatrix3::translation(-target.x, -target.y, -target.z);
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
            scroll: 0.0,
        }
    }

    pub fn process_keyboard(
        &mut self,
        keycode: &VirtualKeyCode,
        state: ElementState,
        window: &Window,
    ) -> bool {
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
            VirtualKeyCode::W | VirtualKeyCode::Up | VirtualKeyCode::Comma => {
                self.is_forward_pressed = is_pressed;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.is_left_pressed = is_pressed;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down | VirtualKeyCode::O => {
                self.is_backward_pressed = is_pressed;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right | VirtualKeyCode::E => {
                self.is_right_pressed = is_pressed;
                true
            }

            VirtualKeyCode::R => {
                self.is_counter_clock_pressed = is_pressed;
                true
            }
            VirtualKeyCode::T => {
                self.is_clock_pressed = is_pressed;
                true
            }
            VirtualKeyCode::Escape => {
                if is_pressed {
                    self.mouse_captured = !self.mouse_captured;
                    let _ = window.set_cursor_grab(self.mouse_captured);
                    window.set_cursor_visible(!self.mouse_captured);
                }

                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, camera: &mut Camera, dx: f64, dy: f64) {
        if dx.is_normal() {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(
                    0.,
                    -self.sensitivity * dx as f32,
                    0.,
                ));
        }

        if dy.is_normal() {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(
                    self.sensitivity * dy as f32,
                    0.,
                    0.,
                ));
        }
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

        if self.is_forward_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(dt * self.speed * -Vector3::z()).into());
        }
        if self.is_backward_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(dt * self.speed * Vector3::z()).into());
        }

        if self.is_right_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(dt * self.speed * -Vector3::x()).into());
        }
        if self.is_left_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(dt * self.speed * Vector3::x()).into());
        }

        if self.is_up_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(dt * self.speed * Vector3::y()).into());
        }
        if self.is_down_pressed {
            camera
                .view_matrix
                .append_translation_mut(&(dt * self.speed * -Vector3::y()).into());
        }
        if self.is_clock_pressed {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(
                    0.,
                    0.,
                    dt * self.sensitivity * -0.05,
                ));
        }
        if self.is_counter_clock_pressed {
            camera
                .view_matrix
                .append_rotation_mut(&Rotation3::from_euler_angles(
                    0.,
                    0.,
                    dt * self.sensitivity * 0.05,
                ));
        }
    }
}
