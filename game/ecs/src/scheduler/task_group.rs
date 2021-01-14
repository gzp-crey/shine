use crate::{
    scheduler::{IntoSystem, System, Task},
    ECSError,
};
use std::{ops::Deref, sync::Arc};

pub trait Runnable {
    /// Prepare and lock for run
    fn lock(&self) -> Result<(), ECSError>;
    /// Get the system to run. Unsafe as locking is not guarded.
    unsafe fn system(&self) -> &mut dyn System;
    /// Release after run
    fn unlock(&self);
}

/// A group of tasks
#[derive(Default)]
pub struct TaskGroup {
    tasks: Vec<Arc<dyn Runnable>>,
}

impl TaskGroup {
    pub fn from_tasks<I>(&mut self, tasks: I) -> TaskGroup
    where
        I: IntoIterator,
        I::Item: Into<Arc<dyn Runnable>>,
    {
        TaskGroup {
            tasks: tasks.into_iter().map(|x| x.into()).collect(),
        }
    }

    /*pub fn add<R, S: IntoSystem<R>>(&mut self, sys: S) -> Result<(), ECSError> {
        let system = Task::new(sys)?;
        self.add_task(system.into());
        Ok(())
    }*/

    pub fn add(&mut self, sys: Arc<dyn Runnable>) {
        self.tasks.push(sys);
    }
}

impl Deref for TaskGroup {
    type Target = [Arc<dyn Runnable>];

    fn deref(&self) -> &[Arc<dyn Runnable>] {
        &self.tasks
    }
}
