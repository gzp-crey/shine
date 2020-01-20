use azure_sdk_core::errors::AzureError;

pub fn is_conflict_error(err: &AzureError) -> bool {
    if let AzureError::UnexpectedHTTPResult(ref res) = err {
        if res.status_code() == 409 {
            return true;
        }
    }
    false
}
