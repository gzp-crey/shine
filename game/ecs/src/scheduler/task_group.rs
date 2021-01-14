use crate::{
    scheduler::{System, Task},
    ECSError,
};
use std::{ops::Deref, sync::Arc};

pub trait Runnable {
    /// Prepare and lock for run
    fn lock(&self) -> Result<(), ECSError>;
    /// Get the system to run.
    /// # Safety
    ///  Calling this function while the object is not locked may result in undefined behaviour.
    #[allow(clippy::mut_from_ref)]
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
    pub fn from_tasks<I>(tasks: I) -> TaskGroup
    where
        I: IntoIterator,
        I::Item: Into<Arc<dyn Runnable>>,
    {
        let mut task = TaskGroup::default();
        task.add_tasks(tasks);
        task
    }

    pub fn add_task<T: Into<Arc<dyn Runnable>>>(&mut self, task: T) {
        self.tasks.push(task.into());
    }

    pub fn add_tasks<I>(&mut self, tasks: I)
    where
        I: IntoIterator,
        I::Item: Into<Arc<dyn Runnable>>,
    {
        self.tasks.extend(tasks.into_iter().map(|t| t.into()));
    }

    pub fn from_system<S: 'static + System>(&mut self, sys: S) -> TaskGroup {
        let mut task = TaskGroup::default();
        task.add_system(sys);
        task
    }

    pub fn add_system<S: 'static + System>(&mut self, sys: S) {
        let task = Task::new(sys);
        let task: Arc<dyn Runnable> = task;
        self.add_task(task);
    }
}

impl Deref for TaskGroup {
    type Target = [Arc<dyn Runnable>];

    fn deref(&self) -> &[Arc<dyn Runnable>] {
        &self.tasks
    }
}
