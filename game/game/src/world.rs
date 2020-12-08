use crate::app::AppError;
use shine_ecs::{resources::Resources, scheduler::Schedule};
use std::collections::HashMap;

#[derive(Default)]
pub struct World {
    pub resources: Resources,
    schedules: HashMap<String, Schedule>,
}

impl World {
    pub fn add_stage(&mut self, stage: &str, schedule: Schedule) {
        let _ = self.schedules.insert(stage.into(), schedule);
    }

    pub fn remove_stage(&mut self, stage: &str) {
        let _ = self.schedules.remove(stage);
    }

    pub fn clear_stages(&mut self) {
        self.schedules.clear();
    }

    pub fn run_stage(&mut self, stage: &str) -> Result<(), AppError> {
        if let Some(stage) = self.schedules.get_mut(stage) {
            stage.run(&self.resources).map_err(AppError::TaskError)?;
        }
        Ok(())
    }
}
