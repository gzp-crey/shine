use crate::app::AppError;
use shine_ecs::{
    resources::Resources,
    scheduler::{Scheduler, TaskGroup},
};
use std::collections::HashMap;

#[derive(Default)]
pub struct World {
    pub resources: Resources,
    scheduler: Scheduler,
    stages: HashMap<String, TaskGroup>,
}

impl World {
    pub fn add_stage(&mut self, stage: &str, tasks: TaskGroup) {
        let _ = self.stages.insert(stage.into(), tasks);
    }

    pub fn remove_stage(&mut self, stage: &str) {
        let _ = self.stages.remove(stage);
    }

    pub fn clear_stages(&mut self) {
        self.stages.clear();
    }

    pub fn run_stage(&mut self, stage: &str) -> Result<(), AppError> {
        if let Some(stage) = self.stages.get(stage) {
            self.scheduler
                .run(&self.resources, stage)
                .map_err(AppError::TaskError)?;
        }
        Ok(())
    }
}
