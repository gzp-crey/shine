use azure_sdk_core::errors::AzureError;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use std::str::Utf8Error;

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

pub fn encode_safe_key(input: &str) -> String {
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string().replace("%", "@")
}

pub fn decode_safe_key(input: &str) -> Result<String, Utf8Error> {
    let input = input.replace("@", "%");
    percent_decode_str(&input).decode_utf8().map(|d| d.to_string())
}

pub mod table_storage {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct EmptyData {}
}
