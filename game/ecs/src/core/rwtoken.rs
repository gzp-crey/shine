// Based on hecs::AtomicBorrow (https://github.com/Ralith/hecs/blob/master/src/borrow.rs)

use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RWTokenError {
    #[error("Target already borrowed as immutable")]
    AlreadyReadLocked,

    #[error("Target already borrowed as mutable")]
    AlreadyWriteLocked,
}

/// Marker bit for the write_lock
const WRITE_BIT: usize = !(usize::max_value() >> 1);

/// Simmilar to RWLock, but the resource guarded is not part of the
/// structure.
pub struct RWToken(AtomicUsize);

impl Default for RWToken {
    fn default() -> Self {
        Self::new()
    }
}

impl RWToken {
    pub fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    pub fn new_write_locked() -> Self {
        Self(AtomicUsize::new(WRITE_BIT))
    }

    pub fn try_read_lock(&self) -> Result<(), RWTokenError> {
        let value = self.0.fetch_add(1, Ordering::Acquire).wrapping_add(1);
        if value == 0 {
            core::panic!("Counter wrapped, this borrow is invalid!")
        }
        if value & WRITE_BIT != 0 {
            self.0.fetch_sub(1, Ordering::Release);
            Err(RWTokenError::AlreadyWriteLocked)
        } else {
            Ok(())
        }
    }

    pub fn read_unlock(&self) {
        let value = self.0.fetch_sub(1, Ordering::Release);
        debug_assert_ne!(value, 0, "unbalanced releases");
        debug_assert_eq!(
            value & WRITE_BIT,
            0,
            "shared release of unique borrow, write_lock - read_unlock"
        );
    }

    #[inline]
    pub fn is_read_lock(&self) -> bool {
        let value = self.0.load(Ordering::Relaxed);
        value != 0 && (value & WRITE_BIT == 0)
    }

    pub fn try_write_lock(&self) -> Result<(), RWTokenError> {
        self.0
            .compare_exchange(0, WRITE_BIT, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|value| {
                if value & WRITE_BIT == 0 {
                    RWTokenError::AlreadyReadLocked
                } else {
                    RWTokenError::AlreadyWriteLocked
                }
            })?;
        Ok(())
    }

    pub fn write_unlock(&self) {
        let value = self.0.fetch_and(!WRITE_BIT, Ordering::Release);
        debug_assert_ne!(
            value & WRITE_BIT,
            0,
            "unique release of shared borrow, read_lock - write_unlock"
        );
    }

    #[inline]
    pub fn is_write_lock(&self) -> bool {
        let value = self.0.load(Ordering::Relaxed);
        value & WRITE_BIT != 0
    }
}
