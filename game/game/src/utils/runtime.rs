use std::future::Future;

pub trait TaskSpawner {
    fn spawn_background_task<F: 'static + Send + Future<Output = ()>>(&mut self, fut: F);
}

pub struct Runtime;

impl Runtime {
    pub fn new() -> Runtime {
        Runtime
    }
}

#[cfg(feature = "native")]
impl TaskSpawner for Runtime {
    fn spawn_background_task<F: 'static + Send + Future<Output = ()>>(&mut self, fut: F) {
        use tokio::spawn;
        spawn(fut);
    }
}

#[cfg(feature = "wasm")]
impl TaskSpawner for Runtime {
    fn spawn_background_task<F: 'static + Future<Output = ()>>(&mut self, fut: F) {
        use wasm_bindgen_futures::spawn_local;
        spawn_local(fut);
    }
}
