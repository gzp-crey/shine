use azure_sdk_core::errors::AzureError;
use std::future::Future;
use std::time::Duration;
use tokio::time::delay_for;

pub enum BackoffError<E> {
    /// Action failed a new attempt is required
    Retry(BackoffContext),

    /// Action failed with an error
    Action(E),
}

impl<E> From<E> for BackoffError<E> {
    fn from(err: E) -> BackoffError<E> {
        BackoffError::Action(err)
    }
}

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
    pub fn retry(&self) -> usize {
        self.retry
    }

    /// Perform an exponential update for the timeout
    pub fn exponential_timeout(&mut self) {
        self.time *= 2.
    }

    /// Perform a retry on azure conflict error
    pub fn retry_on_azure_conflict(mut self, err: AzureError) -> Result<BackoffContext, AzureError> {
        match err {
            AzureError::UnexpectedHTTPResult(e) if e.status_code() == 412 => {
                self.exponential_timeout();
                Ok(self)
            }
            e => Err(e),
        }
    }
}

pub async fn retry<T, E, A, F>(mut context: BackoffContext, action: F) -> Result<Result<T, BackoffContext>, E>
where
    F: Fn(BackoffContext) -> A,
    A: Future<Output = Result<Result<T, BackoffContext>, E>>,
{
    loop {
        context = match action(context).await? {
            Ok(v) => return Ok(Ok(v)),
            Err(context) => context,
        };

        if context.is_last_try() {
            log::warn!("Backoff retry reached maximum iteration ({})", context.retry);
            return Ok(Err(context));
        }

        delay_for(Duration::from_micros(context.time as u64)).await;
        context.retry += 1;
    }
}
