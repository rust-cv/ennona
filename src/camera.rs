use nalgebra::{Matrix4, Point3, Vector3};

pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::face_towards(&self.eye, &self.target, &self.up);

        let perspective = Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar);

        perspective * view
    }
}
