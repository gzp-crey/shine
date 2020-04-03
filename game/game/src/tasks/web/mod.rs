use crate::tasks::TaskEngine;
use std::future::Future;
use wasm_bindgen_futures::spawn_local;

pub struct Engine;

impl Engine {
    pub fn new() -> Engine {
        Engine
    }
}

impl TaskEngine for Engine {
    fn spawn_background_task<F: 'static + Future<Output = ()>>(&mut self, fut: F) {
        spawn_local(fut);
    }
}
