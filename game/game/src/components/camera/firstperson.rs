use crate::components::camera::Camera;
use nalgebra::{Isometry3, Perspective3, Point3, Translation3, UnitQuaternion, Vector3};

/// First person camera
#[derive(Debug)]
pub struct FirstPerson {
    eye: Point3<f32>,
    target_distance: f32,
    yaw: f32,
    pitch: f32,
    roll: f32,

    perspective: Perspective3<f32>,
    view: Isometry3<f32>,
    inverse_view: Isometry3<f32>,
}

impl Default for FirstPerson {
    fn default() -> Self {
        let eye = (0., 0., 1.);

        let mut camera = FirstPerson {
            eye: Point3::new(eye.0, eye.1, eye.2),
            target_distance: 1.,
            yaw: 0.,
            pitch: 0.,
            roll: 0.,

            perspective: Perspective3::new(1., 60.0_f32.to_radians(), 0.1, 1000.),
            view: Isometry3::identity(),
            inverse_view: Isometry3::identity(),
        };
        camera.update();
        camera
    }
}

impl FirstPerson {
    pub fn get_eye(&self) -> &Point3<f32> {
        &self.eye
    }

    pub fn get_target(&self) -> Point3<f32> {
        self.eye + self.get_forward() * self.target_distance
    }

    pub fn get_forward(&self) -> Vector3<f32> {
        self.inverse_view.transform_vector(&Vector3::new(0., 0., -1.))
    }

    pub fn get_up(&self) -> Vector3<f32> {
        self.inverse_view.transform_vector(&Vector3::new(0., 1., 0.))
    }

    pub fn get_side(&self) -> Vector3<f32> {
        self.inverse_view.transform_vector(&Vector3::new(1., 0., 0.))
    }

    pub fn set_view(&mut self, _view: Isometry3<f32>) {
        unimplemented!()
    }

    pub fn set_roll(&mut self, angle: f32) {
        self.roll = angle;
        self.update();
    }

    pub fn roll(&mut self, angle: f32) {
        self.roll += angle;
        self.update();
    }

    pub fn set_yaw(&mut self, angle: f32) {
        self.yaw = angle;
        self.update();
    }

    pub fn yaw(&mut self, angle: f32) {
        self.yaw += angle;
        self.update();
    }

    pub fn set_pitch(&mut self, angle: f32) {
        self.pitch = angle;
        self.update();
    }

    pub fn pitch(&mut self, angle: f32) {
        self.pitch += angle;
        self.update();
    }

    pub fn move_forward(&mut self, dist: f32) {
        let tr = self.get_forward() * dist;
        self.eye += tr;
        self.update();
    }

    pub fn move_side(&mut self, dist: f32) {
        let tr = self.get_side() * dist;
        self.eye += tr;
        self.update();
    }

    pub fn move_up(&mut self, dist: f32) {
        let tr = self.get_up() * dist;
        self.eye += tr;
        self.update();
    }

    pub fn set_perspective(&mut self, perspective: Perspective3<f32>) {
        self.perspective = perspective;
        //self.update();
    }

    pub fn set_perspective_parameters(&mut self, aspect: f32, fovy: f32, znear: f32, zfar: f32) {
        self.perspective = Perspective3::new(aspect, fovy, znear, zfar);
        //self.update();
    }

    pub fn znear(&self) -> f32 {
        self.perspective.znear()
    }

    pub fn zfar(&self) -> f32 {
        self.perspective.znear()
    }

    pub fn image_aspect(&self) -> f32 {
        self.perspective.aspect()
    }

    pub fn fovy(&self) -> f32 {
        self.perspective.fovy()
    }

    /*pub fn fovx(&self) -> f32 {
        self.perspective.fovx()
    }*/

    //pub fn fov_zoom(&mut self, ratio: f32) {}

    //pub fn set_perspective_view(view: Isometry3<f32>, perspective:Perspective3<f32> )

    fn clamp_angles(&mut self) {
        use std::f32::consts::PI;

        self.yaw %= PI * 2.;
        self.roll %= PI * 2.;

        let pitch_limit = PI - 0.001;
        if self.pitch < -pitch_limit {
            self.pitch = -pitch_limit;
        }
        if self.pitch > pitch_limit {
            self.pitch = pitch_limit;
        }
    }

    fn update(&mut self) {
        self.clamp_angles();

        let rot_yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw);
        let rot_pitch = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch);
        let rot_roll = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), self.roll);
        let rot = rot_yaw * rot_pitch * rot_roll;
        let trans = Translation3::from(self.get_eye().coords);

        self.inverse_view = Isometry3::from_parts(trans, rot);
        self.view = self.inverse_view.inverse();
    }
}

impl Camera for FirstPerson {
    fn get_view(&self) -> Isometry3<f32> {
        self.view
    }

    fn get_inverse_view(&self) -> Isometry3<f32> {
        self.inverse_view
    }

    fn get_perspective(&self) -> Perspective3<f32> {
        self.perspective
    }

    fn set<C: Camera>(&mut self, c: &C) {
        self.set_view(c.get_view());
        self.set_perspective(c.get_perspective());
    }
}
