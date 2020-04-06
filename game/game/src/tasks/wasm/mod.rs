use crate::tasks::TaskSpawner;
use std::future::Future;
use wasm_bindgen_futures::spawn_local;

pub struct TaskEngine;

impl TaskEngine {
    pub fn new() -> TaskEngine {
        TaskEngine
    }
}

impl TaskSpawner for TaskEngine {
    fn spawn_background_task<F: 'static + Future<Output = ()>>(&mut self, fut: F) {
        spawn_local(fut);
    }
}
