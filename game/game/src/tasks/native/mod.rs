use crate::tasks::TaskSpawner;
use std::future::Future;
use tokio;

pub struct TaskEngine;

impl TaskEngine {
    pub fn new() -> TaskEngine {
        TaskEngine
    }
}

impl TaskSpawner for TaskEngine {
    fn spawn_background_task<F: 'static + Send + Future<Output = ()>>(&mut self, fut: F) {
        tokio::spawn(fut);
    }
}
