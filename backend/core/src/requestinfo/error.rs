use actix_web::{http::header, ResponseError};
use std::{fmt, str};

/// Possible errors while parsing `Authorization` header.
#[derive(Debug)]
pub enum RequestInfoError {
    /// Missing authentication header
    Header,
    /// Header value is malformed
    Invalid,
    /// Authentication scheme is missing
    MissingScheme,
    /// Required authentication field is missing
    MissingField(&'static str),
    /// Unable to convert header into the str
    ToStrError(header::ToStrError),
    /// Malformed base64 string
    Base64DecodeError(data_encoding::DecodeError),
    /// Malformed UTF-8 string
    Utf8Error(str::Utf8Error),
}

impl fmt::Display for RequestInfoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RequestInfoError::Header => write!(f, "Invalid header"),
            RequestInfoError::Invalid => write!(f, "Invalid header value"),
            RequestInfoError::MissingScheme => write!(f, "Missing scheme"),
            RequestInfoError::MissingField(_) => write!(f, "Missing header field"),
            RequestInfoError::ToStrError(e) => write!(f, "{}", e),
            RequestInfoError::Base64DecodeError(e) => write!(f, "{}", e),
            RequestInfoError::Utf8Error(e) => write!(f, "{}", e),
        }
    }
}

impl ResponseError for RequestInfoError {}

impl From<header::ToStrError> for RequestInfoError {
    fn from(e: header::ToStrError) -> Self {
        RequestInfoError::ToStrError(e)
    }
}
impl From<data_encoding::DecodeError> for RequestInfoError {
    fn from(e: data_encoding::DecodeError) -> Self {
        RequestInfoError::Base64DecodeError(e)
    }
}
impl From<str::Utf8Error> for RequestInfoError {
    fn from(e: str::Utf8Error) -> Self {
        RequestInfoError::Utf8Error(e)
    }
}
