mod error;
mod executor;
mod strategy;

pub use self::error::*;
pub use self::executor::*;
pub use self::strategy::*;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

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
