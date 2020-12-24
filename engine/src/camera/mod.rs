use nalgebra::{
    one, Isometry3, Matrix4, Point3, Translation3, Unit, UnitQuaternion, Vector2, Vector3,
};
use ncollide3d::query::Ray;
use projection::Projection;

pub mod flycam;
pub mod projection;

pub struct Camera {
    pub eye: Point3<f32>,
    yaw: f32,
    pitch: f32,

    projection: Projection,
    view: Matrix4<f32>,

    coord_system: CoordSystemRh,

    pub view_proj: Matrix4<f32>,
}

impl Camera {
    pub fn new(eye: Point3<f32>, at: Point3<f32>, projection: Projection) -> Self {
        let mut res = Self {
            eye,
            yaw: 0.0,
            pitch: 0.0,
            projection,
            coord_system: CoordSystemRh::from_up_axis(Vector3::y_axis()),
            view_proj: one(),
            view: one(),
        };

        res.look_at(eye, at);

        res
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    pub fn look_at(&mut self, eye: Point3<f32>, at: Point3<f32>) -> &mut Self {
        let dist = (eye - at).norm();

        let view_eye = self.coord_system.rotation_to_y_up * eye;
        let view_at = self.coord_system.rotation_to_y_up * at;
        let pitch = ((view_at.y - view_eye.y) / dist).acos();
        let yaw = (view_at.z - view_eye.z).atan2(view_at.x - view_eye.x);

        self.eye = eye;
        self.yaw = yaw;
        self.pitch = pitch;
        self.update_viewproj()
    }

    pub fn at(&self) -> Point3<f32> {
        let view_eye = self.coord_system.rotation_to_y_up * self.eye;
        let ax = view_eye.x + self.yaw.cos() * self.pitch.sin();
        let ay = view_eye.y + self.pitch.cos();
        let az = view_eye.z + self.yaw.sin() * self.pitch.sin();
        self.coord_system.rotation_to_y_up.inverse() * Point3::new(ax, ay, az)
    }

    pub fn update_viewproj(&mut self) -> &mut Self {
        self.view = self.view_transform().to_homogeneous();
        self.view_proj = self.projection.as_matrix() * self.view;
        self
    }

    fn update_restrictions(&mut self) {
        if self.pitch <= 0.01 {
            self.pitch = 0.01
        }

        let _pi: f32 = std::f32::consts::PI;
        if self.pitch > _pi - 0.01 {
            self.pitch = _pi - 0.01
        }
    }

    pub fn view_transform(&self) -> Isometry3<f32> {
        Isometry3::look_at_rh(&self.eye, &self.at(), &self.coord_system.up_axis)
    }

    pub fn ray(&self) -> Ray<f32> {
        Ray::new(self.eye, self.observer_frame() * Vector3::z())
    }

    #[inline]
    pub fn rotate_mut(&mut self, disp: &Vector2<f32>) {
        self.yaw += disp.x;
        self.pitch += disp.y;

        self.update_restrictions();
        self.update_viewproj();
    }

    #[inline]
    pub fn translate_mut(&mut self, t: &Translation3<f32>) {
        let new_eye = t * self.eye;

        self.set_eye(new_eye);
    }

    #[inline]
    fn set_eye(&mut self, eye: Point3<f32>) {
        self.eye = eye;
        self.update_viewproj();
    }

    #[inline]
    fn observer_frame(&self) -> Isometry3<f32> {
        Isometry3::face_towards(&self.eye, &self.at(), &self.coord_system.up_axis)
    }
}

#[derive(Clone, Copy, Debug)]
struct CoordSystemRh {
    up_axis: Unit<Vector3<f32>>,
    rotation_to_y_up: UnitQuaternion<f32>,
}

impl CoordSystemRh {
    #[inline]
    fn from_up_axis(up_axis: Unit<Vector3<f32>>) -> Self {
        let rotation_to_y_up = UnitQuaternion::rotation_between_axis(&up_axis, &Vector3::y_axis())
            .unwrap_or_else(|| {
                UnitQuaternion::from_axis_angle(&Vector3::x_axis(), std::f32::consts::PI)
            });
        Self {
            up_axis,
            rotation_to_y_up,
        }
    }
}
