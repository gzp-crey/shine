use actix_web::{http::header, ResponseError};
use std::{fmt, str};

/// Possible errors while parsing `Authorization` header.
#[derive(Debug)]
pub enum AuthHeaderError {
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

impl fmt::Display for AuthHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthHeaderError::Header => write!(f, "Invalid authentication header"),
            AuthHeaderError::Invalid => write!(f, "Invalid header value"),
            AuthHeaderError::MissingScheme => write!(f, "Missing authorization scheme"),
            AuthHeaderError::MissingField(_) => write!(f, "Missing header field"),
            AuthHeaderError::ToStrError(e) => write!(f, "{}", e),
            AuthHeaderError::Base64DecodeError(e) => write!(f, "{}", e),
            AuthHeaderError::Utf8Error(e) => write!(f, "{}", e),
        }
    }
}

impl ResponseError for AuthHeaderError {}

impl From<header::ToStrError> for AuthHeaderError {
    fn from(e: header::ToStrError) -> Self {
        AuthHeaderError::ToStrError(e)
    }
}
impl From<data_encoding::DecodeError> for AuthHeaderError {
    fn from(e: data_encoding::DecodeError) -> Self {
        AuthHeaderError::Base64DecodeError(e)
    }
}
impl From<str::Utf8Error> for AuthHeaderError {
    fn from(e: str::Utf8Error) -> Self {
        AuthHeaderError::Utf8Error(e)
    }
}
