use super::{Backoff, BackoffError};
use std::future::Future;
use tokio::time::delay_for;

pub async fn async_executor<B, T, E, A, F>(backoff: &mut B, mut action: A) -> Result<T, E>
where
    B: Backoff,
    A: FnMut(usize) -> F,
    F: Future<Output = Result<T, BackoffError<E>>>,
{
    let mut retry: usize = 0;
    loop {
        let fut = action(retry);
        let timeout = match fut.await {
            Ok(v) => return Ok(v),
            Err(BackoffError::Permanent(e)) => return Err(e),
            Err(BackoffError::Transient(e)) => match Backoff::next_backoff(backoff) {
                Some(time) => time,
                None => return Err(e),
            },
        };

        delay_for(timeout).await;
        retry += 1;
    }
}
