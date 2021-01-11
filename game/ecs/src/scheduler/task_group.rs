use crate::{
    scheduler::{IntoSystem, Task},
    ECSError,
};
use std::ops::Deref;

/// A group of systems
#[derive(Default)]
pub struct TaskGroup {
    systems: Vec<Task>,
}

impl TaskGroup {
    pub fn add<R, S: IntoSystem<R>>(&mut self, sys: S) -> Result<(), ECSError> {
        let system = Task::new(sys.into_system()?);
        self.add_task(system);
        Ok(())
    }

    pub fn add_task(&mut self, sys: Task) {
        assert!(!sys.is_locked());
        self.systems.push(sys);
    }
}

impl Deref for TaskGroup {
    type Target = [Task];

    fn deref(&self) -> &[Task] {
        &self.systems
    }
}
