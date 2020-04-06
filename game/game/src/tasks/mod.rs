use std::future::Future;

pub trait TaskSpawner {
    fn spawn_background_task<F: 'static + Send + Future<Output = ()>>(&mut self, fut: F);
}

#[cfg(feature = "native")]
#[path = "native/mod.rs"]
mod engine;

#[cfg(feature = "wasm")]
#[path = "wasm/mod.rs"]
mod engine;

pub use self::engine::*;
