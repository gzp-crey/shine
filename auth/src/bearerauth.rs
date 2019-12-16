use actix_web::http::header::{
    HeaderValue, IntoHeaderValue, InvalidHeaderValueBytes,
};

/// Possible errors while parsing `Authorization` header.
///
/// Should not be used directly unless you are implementing
/// your own [authentication scheme](./trait.Scheme.html).
#[derive(Debug)]
pub enum Error {
    /// Header value is malformed
    Invalid,
    /// Authentication scheme is missing
    MissingScheme,
    /// Required authentication field is missing
    MissingField(&'static str),
    /// Unable to convert header into the str
    ToStrError(header::ToStrError),
    /// Malformed base64 string
    Base64DecodeError(base64::DecodeError),
    /// Malformed UTF-8 string
    Utf8Error(str::Utf8Error),
}

/// Credentials for `Bearer` authentication scheme, defined in [RFC6750](https://tools.ietf.org/html/rfc6750)
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct BearerAuth {
    token: Cow<'static, str>,
}

impl BearerAuth {
    /// Creates new `Bearer` credentials with the token provided.
    pub fn new<T>(token: T) -> BearerAuth
    where
        T: Into<Cow<'static, str>>,
    {
        BearerAuth {
            token: token.into(),
        }
    }

    /// Gets reference to the credentials token.
    pub fn token(&self) -> &Cow<'static, str> {
        &self.token
    }
}

impl FromRequest for BearerAuth {
    type Future = Result<Self, Self::Error>;
    type Error = ParseError;

    fn from_request(
        req: &HttpRequest,
        _payload: &mut Payload,
    ) -> Self::Future {
        // "Bearer *" length
        if header.len() < 8 {
            return Err(ParseError::Invalid);
        }

        let mut parts = header.to_str()?.splitn(2, ' ');
        match parts.next() {
            Some(scheme) if scheme == "Bearer" => (),
            _ => return ready(Err(ParseError::MissingScheme)),
        }

        let token = parts.next().ok_or(ParseError::Invalid)?;

        Ok(Bearer {
            token: token.to_string().into(),
        })
    }
}