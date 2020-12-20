use nalgebra::{Matrix4, Perspective3};

#[allow(unused)]
pub struct Projection {
    fovy: f32,
    znear: f32,
    zfar: f32,
    gpu_mat: Matrix4<f32>,
    perspective: Perspective3<f32>,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            fovy,
            znear,
            zfar,
            #[rustfmt::skip]
            gpu_mat: Matrix4::new(
                -1.0, 0.0, 0.0, 0.0,
                0.0, -1.0, 0.0, 0.0,
                0.0, 0.0, 0.5, 0.0,
                0.0, 0.0, 0.5, 1.0,
            ),
            perspective: Perspective3::new(width as f32 / height as f32, fovy, znear, zfar),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.perspective.set_aspect(width as f32 / height as f32);
    }

    pub fn as_matrix(&self) -> Matrix4<f32> {
        self.gpu_mat * self.perspective.as_matrix()
    }
}
