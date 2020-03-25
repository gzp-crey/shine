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
