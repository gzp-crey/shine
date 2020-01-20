use azure_sdk_core::errors::AzureError;

pub fn is_conflict_error(err: &AzureError) -> bool {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 412 => true,
        _ => false,
    }
}

pub fn is_conflict_error2(err: &AzureError) -> bool {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 409 => true,
        _ => false,
    }
}
