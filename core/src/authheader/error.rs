use actix_web::{http::header, ResponseError};
use std::{fmt, str};

/// Possible errors while parsing `Authorization` header.
#[derive(Debug)]
pub enum Error {
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Header => write!(f, "Invalid authentication header"),
            Error::Invalid => write!(f, "Invalid header value"),
            Error::MissingScheme => write!(f, "Missing authorization scheme"),
            Error::MissingField(_) => write!(f, "Missing header field"),
            Error::ToStrError(e) => write!(f, "{}", e),
            Error::Base64DecodeError(e) => write!(f, "{}", e),
            Error::Utf8Error(e) => write!(f, "{}", e),
        }
    }
}

impl ResponseError for Error {}

impl From<header::ToStrError> for Error {
    fn from(e: header::ToStrError) -> Self {
        Error::ToStrError(e)
    }
}
impl From<data_encoding::DecodeError> for Error {
    fn from(e: data_encoding::DecodeError) -> Self {
        Error::Base64DecodeError(e)
    }
}
impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Utf8Error(e)
    }
}
