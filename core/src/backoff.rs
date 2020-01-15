use azure_sdk_core::errors::AzureError;
use std::future::Future;
use std::time::Duration;
use tokio::time::delay_for;

pub enum BackoffError<E> {
    Retry { timeout: f32 },
    Action(E),
    RetryLimit(usize, f32),
}

pub async fn retry<T, E, A, F>(action: F) -> Result<T, BackoffError<E>>
where
    F: Fn(usize, f32) -> A,
    A: Future<Output = Result<T, BackoffError<E>>>,
{
    let mut retry = 0;
    let mut timeout = 0.0;
    loop {
        timeout = match action(retry, timeout).await {
            Err(BackoffError::Retry { timeout }) => timeout,
            e => return e,
        };

        retry += 1;
        delay_for(Duration::from_micros(timeout as u64)).await;
    }
}

pub fn map_azure_error(retry: usize, timeout: f32, err: AzureError) -> BackoffError<AzureError> {
    if retry > 10 {
        BackoffError::RetryLimit(retry, timeout)
    } else {
        match err {
            AzureError::UnexpectedHTTPResult(e) if e.status_code() == 412 => BackoffError::Retry {
                timeout: if retry == 0 { 10. } else { timeout * 2. },
            },
            e => BackoffError::Action(e),
        }
    }
}
