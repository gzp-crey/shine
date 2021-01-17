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

/// Warb a Runnable for more ergonomic use.
#[derive(Clone)]
pub struct TaskItem(Arc<dyn Runnable>);

impl Deref for TaskItem {
    type Target = Arc<dyn Runnable>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S: 'static + System> From<S> for TaskItem {
    fn from(task: S) -> TaskItem {
        let task = Task::new(task);
        let task: Arc<dyn Runnable> = task;
        TaskItem(task)
    }
}

impl<S: 'static + System> From<Arc<Task<S>>> for TaskItem {
    fn from(task: Arc<Task<S>>) -> TaskItem {
        let task: Arc<dyn Runnable> = task;
        TaskItem(task)
    }
}

/// A group of tasks
#[derive(Default)]
pub struct TaskGroup {
    tasks: Vec<TaskItem>,
}

impl TaskGroup {
    pub fn from_task<T: Into<TaskItem>>(task: T) -> TaskGroup {
        TaskGroup {
            tasks: vec![task.into()],
        }
    }

    pub fn add_task<T: Into<TaskItem>>(&mut self, task: T) {
        self.tasks.push(task.into());
    }

    pub fn from_tasks<I, T>(tasks: I) -> TaskGroup
    where
        I: IntoIterator<Item = T>,
        T: Into<TaskItem>,
    {
        TaskGroup {
            tasks: tasks.into_iter().map(|task| task.into()).collect(),
        }
    }

    pub fn add_tasks<I, T>(&mut self, tasks: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<TaskItem>,
    {
        self.tasks.extend(tasks.into_iter().map(|task| task.into()))
    }
}

impl Deref for TaskGroup {
    type Target = [TaskItem];

    fn deref(&self) -> &[TaskItem] {
        &self.tasks
    }
}
