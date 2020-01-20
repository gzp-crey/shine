use super::Backoff;
use std::time::Duration;

/// Backoff policy to try executing operation only once.
pub struct Once;

impl Backoff for Once {
    fn next_backoff(&mut self) -> Option<Duration> {
        None
    }
}

/// Backoff policy to try executing operation with a constant timeout and with a limited retry count.
pub struct ConstantTimeout {
    retry: usize,
    timeout: Duration,
}

impl ConstantTimeout {
    pub fn new(retry_count: usize, timeout: Duration) -> ConstantTimeout {
        ConstantTimeout {
            retry: retry_count,
            timeout: timeout,
        }
    }
}

impl Backoff for ConstantTimeout {
    fn next_backoff(&mut self) -> Option<Duration> {
        if self.retry == 0 {
            None
        } else {
            self.retry -= 1;
            Some(self.timeout)
        }
    }
}

/// Backoff policy to try executing operation with an exponential timeout and with a limited try count.
pub struct Exponential {
    retry: usize,
    timeout: Duration,
}

impl Exponential {
    pub fn new(retry_count: usize, initial_timeout: Duration) -> Exponential {
        Exponential {
            retry: retry_count,
            timeout: initial_timeout,
        }
    }
}

impl Backoff for Exponential {
    fn next_backoff(&mut self) -> Option<Duration> {
        if self.retry == 0 {
            None
        } else {
            let timeout = self.timeout;
            self.retry -= 1;
            self.timeout.mul_f32(1.8);
            Some(timeout)
        }
    }
}
