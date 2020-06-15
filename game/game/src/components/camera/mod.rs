use nalgebra::{Isometry3, Perspective3};

pub trait Camera: 'static + Sync + Send {
    /// Get the view part of projection
    fn get_view(&self) -> Isometry3<f32>;

    /// Get the inverse of the view
    fn get_inverse_view(&self) -> Isometry3<f32> {
        self.get_view().inverse()
    }

    /// Get the perspecive part of the projection
    fn get_perspective(&self) -> Perspective3<f32>;

    /// Switch camera
    fn set<C: Camera>(&mut self, camera: &C);
}

mod firstperson;
pub use self::firstperson::*;
mod projection;
pub use self::projection::*;

pub mod systems;
