use crate::{core::finally, resources::Resources, scheduler::TaskGroup, ECSError};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Scheduler {}

impl Scheduler {
    pub fn run(&mut self, resources: &Resources, tasks: &TaskGroup) -> Result<(), ECSError> {
        for task in tasks.iter() {
            task.lock()?;
            let _unlock_guard = finally(|| task.unlock());
            // safety:
            //  task.lock and _unlock_guard ansures the task can be executed
            let mut system = unsafe { task.system() };
            system.run(resources)?;
        }
        Ok(())
    }
}
