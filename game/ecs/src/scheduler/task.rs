use crate::{core::rwtoken::RWToken, scheduler::System, ECSError};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use super::{IntoSystem, Runnable};

/// Wrapper for system to add required bookeeping and locking for the scheduler.
pub struct Task<S: System> {
    system: UnsafeCell<S>,
    lock: RWToken,
}

impl<S: System> Task<S> {
    pub fn new(system: S) -> Arc<Task<S>> {
        Arc::new(Task {
            system: UnsafeCell::new(system),
            lock: Default::default(),
        })
    }

    pub fn system(&self) -> Result<SystemRef<'_, S>, ECSError> {
        SystemRef::new(&self)
    }
}

impl<S: System> Runnable for Task<S> {
    fn lock(&self) -> Result<(), ECSError> {
        self.lock.try_write_lock().map_err(|_| ECSError::SystemLockError)
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn system(&self) -> &mut dyn System {
        assert!(self.lock.is_write_lock());
        &mut *self.system.get()
    }

    fn unlock(&self) {
        self.lock.write_unlock();
    }
}

/// A guard to reference a sytem and handle related locks.
pub struct SystemRef<'a, S: System> {
    task: &'a Task<S>,
}

impl<'a, S: System> SystemRef<'a, S> {
    fn new(task: &'a Task<S>) -> Result<SystemRef<'a, S>, ECSError> {
        task.lock.try_write_lock().map_err(|_| ECSError::SystemLockError)?;
        Ok(SystemRef { task })
    }
}

impl<'a, S: System> Drop for SystemRef<'a, S> {
    fn drop(&mut self) {
        self.task.lock.write_unlock();
    }
}

impl<'a, S: System> Deref for SystemRef<'a, S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        assert!(self.task.lock.is_write_lock());
        // safety:
        //  lock ensures the appropriate access
        unsafe { &*self.task.system.get() }
    }
}

impl<'a, S: System> DerefMut for SystemRef<'a, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(self.task.lock.is_write_lock());
        // safety:
        //  lock ensures the appropriate access
        unsafe { &mut *self.task.system.get() }
    }
}
