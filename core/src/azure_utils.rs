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

    #[derive(Debug, Serialize, Deserialize)]
    pub struct EmptyData {}
}

pub const DATE_TIME_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

pub mod serde_with_opt_datetime {
    use super::DATE_TIME_FORMAT;
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(date) = date {
            let s = format!("{}", date.format(DATE_TIME_FORMAT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_str("")
        }
    }

    pub fn deserialize<'d, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'d>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "" {
            Ok(None)
        } else {
            let date = Utc.datetime_from_str(&s, DATE_TIME_FORMAT).map_err(Error::custom)?;
            Ok(Some(date))
        }
    }
}

pub mod serde_with_datetime {
    use super::DATE_TIME_FORMAT;
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(DATE_TIME_FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'d, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'d>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, DATE_TIME_FORMAT).map_err(Error::custom)
    }
}
