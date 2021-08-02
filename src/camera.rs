use nalgebra::{IsometryMatrix3, Matrix4, Point3, Vector3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct Camera {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub view_matrix: IsometryMatrix3<f32>
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {

        let perspective = Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar);

        perspective * self.view_matrix.to_matrix()
    }
}

pub struct CameraController {
    pub speed: f32,
    pub is_up_pressed: bool,
    pub is_down_pressed: bool,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent<'_>) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
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
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {

        if self.is_forward_pressed {
            camera.view_matrix = IsometryMatrix3::translation(0., 0., 1.) * camera.view_matrix;
        }
        if self.is_backward_pressed {
            camera.view_matrix = IsometryMatrix3::translation(0., 0., -1.) * camera.view_matrix;
        }

        let right = forward_norm.cross(&camera.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so 
            // that it doesn't change. The eye therefore still 
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
    }
}

