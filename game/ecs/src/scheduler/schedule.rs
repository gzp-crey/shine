use crate::{
    resources::Resources,
    scheduler::{IntoSystem, TaskGroup},
    ECSError,
};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Schedule {
    task_group: TaskGroup,
}

impl Schedule {
    pub fn schedule<R, Func: IntoSystem<R>>(&mut self, func: Func) -> Result<(), ECSError> {
        self.task_group.add(func)
    }

    pub fn run(&mut self, resources: &Resources) -> Result<(), ECSError> {
        for task in self.task_group.iter() {
            let mut system = task.system()?;
            system.run(resources)?;
        }
        Ok(())
    }
}
