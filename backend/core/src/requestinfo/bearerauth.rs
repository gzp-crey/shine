use super::RequestInfoError;
use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};

/// Credentials for `Bearer` authentication scheme, defined in [RFC6750](https://tools.ietf.org/html/rfc6750)
#[derive(Clone)]
pub struct BearerAuth {
    token: String,
}

impl BearerAuth {
    /// Creates new `Bearer` credentials with the token provided.
    pub fn new<T>(token: T) -> BearerAuth
    where
        T: Into<String>,
    {
        BearerAuth { token: token.into() }
    }

    pub fn from_header(header: &header::HeaderValue) -> Result<Self, RequestInfoError> {
        // "Bearer *" length
        if header.len() < 8 {
            return Err(RequestInfoError::Invalid);
        }

        let mut parts = header.to_str()?.splitn(2, ' ');
        match parts.next() {
            Some(scheme) if scheme == "Bearer" => (),
            _ => return Err(RequestInfoError::MissingScheme),
        }

        let token = parts.next().ok_or(RequestInfoError::Invalid)?;

        Ok(BearerAuth {
            token: token.to_string().into(),
        })
    }

    /// Gets reference to the credentials token.
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl FromRequest for BearerAuth {
    type Config = ();
    type Error = RequestInfoError;
    type Future = Ready<Result<Self, RequestInfoError>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let header = match req.headers().get(header::AUTHORIZATION) {
            None => return err(RequestInfoError::Header),
            Some(header) => header,
        };

        match BearerAuth::from_header(header) {
            Ok(auth) => ok(auth),
            Err(e) => err(e),
        }
    }
}
