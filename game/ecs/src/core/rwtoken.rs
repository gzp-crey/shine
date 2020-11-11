use std::sync::atomic::{self, AtomicIsize};

pub enum RWTokenError {
    AlreadyReadLocked,
    AlreadyWriteLocked,
}

/// Simmilar to RLock, but the resource guarded is not part of the
/// structure.
pub struct RWToken(AtomicIsize);

impl RWToken {
    pub fn new() -> Self {
        Self(AtomicIsize::new(0))
    }

    pub fn new_write_locked() -> Self {
        Self(AtomicIsize::new(-1))
    }

    pub fn try_read_lock(&self) -> Result<(), RWTokenError> {
        loop {
            let read = self.0.load(atomic::Ordering::SeqCst);
            if read < 0 {
                return Err(RWTokenError::AlreadyWriteLocked);
            }

            if self.0.compare_and_swap(read, read + 1, atomic::Ordering::SeqCst) == read {
                return Ok(());
            }
        }
    }

    pub fn read_unlock(&self) {
        let p = self.0.fetch_sub(1, atomic::Ordering::SeqCst);
        debug_assert!(p > 0, "Token was not read locked");
    }

    #[inline]
    pub fn is_read(&self) -> bool {
        self.0.load(atomic::Ordering::SeqCst) > 0
    }

    pub fn try_write_lock(&self) -> Result<(), RWTokenError> {
        let borrowed = self.0.compare_and_swap(0, -1, atomic::Ordering::SeqCst);
        match borrowed {
            0 => Ok(()),
            x if x < 0 => Err(RWTokenError::AlreadyWriteLocked),
            _ => Err(RWTokenError::AlreadyReadLocked),
        }
    }

    pub fn write_unlock(&self) {
        let p = self.0.fetch_add(1, atomic::Ordering::SeqCst);
        debug_assert!(p == -1, "Token was not write locked");
    }

    #[inline]
    pub fn is_write(&self) -> bool {
        self.0.load(atomic::Ordering::SeqCst) < 0
    }

    /*#[inline]
    pub unsafe fn is_unlocked(&self) -> bool {
        self.0.load(atomic::Ordering::SeqCst) == 0
    }*/
}
