use crate::camera::Camera;
use nalgebra::{Isometry3, Matrix4, Perspective3, Vector3};

/// Camera used for rendering
#[derive(Debug)]
pub struct Projection {
    view_matrix: Matrix4<f32>,
    inverse_view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    projection_view_matrix: Matrix4<f32>,
}

impl Projection {
    pub fn new() -> Projection {
        Projection {
            view_matrix: Matrix4::identity(),
            inverse_view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            projection_view_matrix: Matrix4::identity(),
        }
    }

    pub fn view_matrix(&self) -> &Matrix4<f32> {
        &self.view_matrix
    }

    pub fn inverse_view_matrix(&self) -> &Matrix4<f32> {
        &self.inverse_view_matrix
    }

    pub fn projection_matrix(&self) -> &Matrix4<f32> {
        &self.projection_matrix
    }

    pub fn projection_view_matrix(&self) -> &Matrix4<f32> {
        &self.projection_view_matrix
    }

    pub fn set_perspective(&mut self, view: &Isometry3<f32>, perspective: &Perspective3<f32>) {
        let flip_y = Matrix4::new_nonuniform_scaling(&Vector3::new(1., -1., 1.));

        self.view_matrix = view.to_homogeneous();
        self.inverse_view_matrix = view.inverse().to_homogeneous();
        self.projection_matrix = flip_y * perspective.as_matrix();
        self.projection_view_matrix = self.projection_matrix * self.view_matrix;
    }

    pub fn set_camera<C: Camera>(&mut self, cam: &C) {
        self.set_perspective(&cam.get_view(), &cam.get_perspective());
    }
}

impl Default for Projection {
    fn default() -> Self {
        Projection::new()
    }
}
