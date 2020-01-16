use std::future::Future;
use std::time::Duration;
use tokio::time::delay_for;

#[derive(Debug, Clone)]
pub struct BackoffContext {
    max_retry: usize,
    retry: usize,
    time: f32,
}

impl BackoffContext {
    pub fn new(max_retry: usize, initial_timeout: f32) -> BackoffContext {
        assert!(max_retry > 0);
        BackoffContext {
            max_retry,
            retry: 0,
            time: initial_timeout,
        }
    }

    /// Return if this is the first attempt to execute the action.
    pub fn is_first_try(&self) -> bool {
        self.retry == 0
    }

    /// Return if this is the last attempt to execute the action.
    pub fn is_last_try(&self) -> bool {
        self.retry == self.max_retry - 1
    }

    /// Return the number of attempts
    pub fn retry_count(&self) -> usize {
        self.retry
    }

    pub fn complete<O, E>(&self, ok: O) -> Result<O, Result<BackoffContext, E>> {
        Ok(ok)
    }

    /// Perform an exponential update for the timeout
    pub fn retry<O, E>(&self) -> Result<O, Result<BackoffContext, E>> {
        Err(Ok(self.clone()))
    }

    pub fn fail<O, E>(&self, err: E) -> Result<O, Result<BackoffContext, E>> {
        Err(Err(err))
    }

    pub fn retry_on_error<E>(&self) -> Result<BackoffContext, E> {
        Ok(self.clone())
    }

    pub fn fail_on_error<E>(&self, err: E) -> Result<BackoffContext, E> {
        Err(err)
    }
}

pub async fn retry<T, E, A, F>(mut context: BackoffContext, action: F) -> Result<T, Result<BackoffContext, E>>
where
    F: Fn(BackoffContext) -> A,
    A: Future<Output = Result<T, Result<BackoffContext, E>>>,
{
    loop {
        context = match action(context).await {
            Err(Ok(context)) => context,
            o => return o,
        };

        if context.is_last_try() {
            log::warn!("Backoff retry reached maximum iteration ({})", context.retry);
            return Err(Ok(context));
        }

        delay_for(Duration::from_micros(context.time as u64)).await;
        context.retry += 1;
        context.time *= 2.;
    }
}

pub async fn retry_err<T, E, A, F, G>(context: BackoffContext, action: F, map_err: G) -> Result<T, E>
where
    F: Fn(BackoffContext) -> A,
    A: Future<Output = Result<T, Result<BackoffContext, E>>>,
    G: Fn(BackoffContext) -> E,
{
    retry::<T, E, A, F>(context, action).await.map_err(|err| match err {
        Err(err) => err,
        Ok(ctx) => map_err(ctx),
    })
}
