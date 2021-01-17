use crate::{core::finally, resources::Resources, scheduler::TaskGroup, ECSError};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Scheduler {}

impl Scheduler {
    pub fn run(&mut self, resources: &Resources, tasks: &TaskGroup) -> Result<(), ECSError> {
        //todo: it is a very draft, no dependency is checked and performs a lot of clone.
        let mut tasks = tasks.iter().rev().cloned().collect::<Vec<_>>();
        while let Some(task) = tasks.pop() {
            task.lock()?;
            let new_tasks = {
                let _unlock_guard = finally(|| task.unlock());
                // safety:
                //  task.lock and _unlock_guard ansures the task can be executed
                let system = unsafe { task.system() };
                system.run(resources)?
            };
            tasks.extend(new_tasks.iter().rev().cloned());
        }
        Ok(())
    }
}
