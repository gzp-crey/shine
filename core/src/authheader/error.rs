use actix_web::{http::header, ResponseError};
use std::{error::Error as StdError, fmt, str};

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
        f.write_str(self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::Header => "Invalid authentication header",
            Error::Invalid => "Invalid header value",
            Error::MissingScheme => "Missing authorization scheme",
            Error::MissingField(_) => "Missing header field",
            Error::ToStrError(e) => e.description(),
            Error::Base64DecodeError(e) => e.description(),
            Error::Utf8Error(e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::ToStrError(e) => Some(e),
            Error::Base64DecodeError(e) => Some(e),
            Error::Utf8Error(e) => Some(e),
            Error::Header | Error::Invalid | Error::MissingScheme | Error::MissingField(_) => None,
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
