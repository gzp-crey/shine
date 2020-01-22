use azure_sdk_core::errors::AzureError;

/// Unexpected status code error for CONFLICT (409)
/// [[RFC7231, Section 6.5.8](https://tools.ietf.org/html/rfc7231#section-6.5.8)]
pub fn is_conflict_error(err: &AzureError) -> bool {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 409 => true,
        _ => false,
    }
}

/// Unexpected status code error for PRECONDITION_FAILED (412)
/// Is is usually used in connection with etag conditions and optimistic concurency.
/// [[RFC7232, Section 4.2](https://tools.ietf.org/html/rfc7232#section-4.2)]
pub fn is_precodition_error(err: &AzureError) -> bool {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 412 => true,
        _ => false,
    }
}

pub mod table_storage {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct EmptyData {}
}
