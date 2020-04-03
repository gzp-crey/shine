use std::future::Future;

pub trait TaskEngine {
    fn spawn_background_task<F: 'static + Future<Output = ()>>(&mut self, fut: F);
}

mod web;
pub use self::web::*;
