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
    pub fn from_system<S: 'static + System>(sys: S) -> TaskGroup {
        Self::from_task(Task::new(sys))
    }

    pub fn add_system<S: 'static + System>(&mut self, sys: S) {
        self.add_task(Task::new(sys));
    }

    pub fn from_task<S: 'static + System>(task: Arc<Task<S>>) -> TaskGroup {
        let mut tg = TaskGroup::default();
        tg.add_task(task);
        tg
    }

    pub fn add_task<S: 'static + System>(&mut self, task: Arc<Task<S>>) {
        let task : Arc<dyn Runnable> = task;
        self.tasks.push(task);
    }

    pub fn from_tasks<I>(tasks: I) -> TaskGroup
    where
        I: IntoIterator,
        I::Item: Into<Arc<dyn Runnable>>,
    {
        let mut tg = TaskGroup::default();
        tg.add_tasks(tasks);
        tg
    }

    pub fn add_tasks<I>(&mut self, tasks: I)
    where
        I: IntoIterator,
        I::Item: Into<Arc<dyn Runnable>>,
    {
        self.tasks.extend(tasks.into_iter().map(|t| t.into()));
    }
}

impl Deref for TaskGroup {
    type Target = [Arc<dyn Runnable>];

    fn deref(&self) -> &[Arc<dyn Runnable>] {
        &self.tasks
    }
}
