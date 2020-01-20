mod executor;
mod strategy;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub use self::executor::*;
pub use self::strategy::*;

/// Error during backoff operation.
pub enum BackoffError<E> {
    /// Permannet error, opertion retry is not possible
    Permanent(E),

    /// Transient error, opertion retry is possible based on the backoff strategy
    Transient(E),
}

impl<E> BackoffError<E> {
    pub fn into_inner(self) -> E {
        match self {
            BackoffError::Permanent(e) => e,
            BackoffError::Transient(e) => e,
        }
    }
}

/// Backoff policy for retrying an operation.
pub trait Backoff {
    /// The time to wait before operation is retried. To stop retry, None shall be returned.
    fn next_backoff(&mut self) -> Option<Duration>;

    /// Execute an async function using the backoff strategy.
    fn async_execute<'s, A, T, E, F>(&'s mut self, action: A) -> Pin<Box<dyn Future<Output = Result<T, E>> + 's>>
    where
        Self: Sized,
        A: 's + FnMut(usize) -> F,
        F: 's + Future<Output = Result<T, BackoffError<E>>>,
        T: 's,
        E: 's,
    {
        Box::pin(executor::async_executor::<Self, T, E, A, F>(self, action))
    }
}
