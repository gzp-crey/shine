use crate::{core::rwtoken::RWToken, scheduler::System, ECSError};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

struct Inner {
    system: UnsafeCell<Box<dyn System>>,
    lock: RWToken,
}

impl Inner {
    fn lock(&self) -> Result<(), ECSError> {
        self.lock.try_write_lock().map_err(|_| ECSError::SystemLockError)?;
        Ok(())
    }

    fn is_locked(&self) -> bool {
        self.lock.is_write_lock()
    }

    #[allow(clippy::mut_from_ref)]
    fn unchecked_system(&self) -> &mut dyn System {
        assert!(self.lock.is_write_lock());
        // safety:
        //  lock ensures the appropriate access
        unsafe { &mut **self.system.get() }
    }

    fn unlock(&self) {
        self.lock.write_unlock();
    }
}

/// Wrapper for system to add required bookeeping and locking for the scheduler.
#[derive(Clone)]
pub struct Task {
    inner: Arc<Inner>,
}

impl Task {
    pub fn new(system: Box<dyn System>) -> Task {
        Task {
            inner: Arc::new(Inner {
                system: UnsafeCell::new(system),
                lock: Default::default(),
            }),
        }
    }

    pub fn lock(&self) -> Result<(), ECSError> {
        self.inner.lock()
    }

    #[allow(clippy::mut_from_ref)]
    pub fn unchecked_system(&self) -> &mut dyn System {
        self.inner.unchecked_system()
    }

    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }

    pub fn unlock(&self) {
        self.inner.unlock();
    }

    pub fn system(&self) -> Result<SystemRef, ECSError> {
        SystemRef::new(&self.inner)
    }
}

pub struct SystemRef<'a> {
    inner: &'a Inner,
}

impl<'a> SystemRef<'a> {
    fn new(inner: &'a Inner) -> Result<SystemRef<'a>, ECSError> {
        inner.lock()?;
        Ok(SystemRef { inner })
    }
}

impl<'a> Drop for SystemRef<'a> {
    fn drop(&mut self) {
        self.inner.unlock();
    }
}

impl<'a> Deref for SystemRef<'a> {
    type Target = dyn System;

    fn deref(&self) -> &Self::Target {
        assert!(self.inner.lock.is_write_lock());
        // safety:
        //  lock ensures the appropriate access
        unsafe { &**self.inner.system.get() }
    }
}

impl<'a> DerefMut for SystemRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(self.inner.lock.is_write_lock());
        // safety:
        //  lock ensures the appropriate access
        unsafe { &mut **self.inner.system.get() }
    }
}
