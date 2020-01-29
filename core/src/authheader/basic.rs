use super::AuthHeaderError;
use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use data_encoding::BASE64;
use futures::future::{err, ok, Ready};
use std::str;

/// Credentials for `Basic` authentication scheme, defined in [RFC 7617](https://tools.ietf.org/html/rfc7617)
#[derive(Debug, Clone)]
pub struct BasicAuth {
    user_id: String,
    password: Option<String>,
}

impl BasicAuth {
    /// Creates `Basic` credentials with provided `user_id` and optional `password`.
    pub fn new<U, P>(user_id: U, password: Option<P>) -> BasicAuth
    where
        U: Into<String>,
        P: Into<String>,
    {
        BasicAuth {
            user_id: user_id.into(),
            password: password.map(Into::into),
        }
    }

    pub fn from_header(header: &header::HeaderValue) -> Result<Self, AuthHeaderError> {
        // "Basic *" length
        if header.len() < 7 {
            return Err(AuthHeaderError::Invalid);
        }

        let mut parts = header.to_str()?.splitn(2, ' ');
        match parts.next() {
            Some(scheme) if scheme == "Basic" => (),
            _ => return Err(AuthHeaderError::MissingScheme),
        }

        let decoded = BASE64.decode(parts.next().ok_or(AuthHeaderError::Invalid)?.as_bytes())?;
        let mut credentials = str::from_utf8(&decoded)?.splitn(2, ':');

        let user_id = credentials
            .next()
            .ok_or(AuthHeaderError::MissingField("user_id"))
            .map(|user_id| user_id.to_string())?;

        let password = credentials
            .next()
            .ok_or(AuthHeaderError::MissingField("password"))
            .map(|password| {
                if password.is_empty() {
                    None
                } else {
                    Some(password.to_string())
                }
            })?;

        Ok(BasicAuth { user_id, password })
    }

    /// Returns client's user-ID.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Returns client's password if provided.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl FromRequest for BasicAuth {
    type Config = ();
    type Error = AuthHeaderError;
    type Future = Ready<Result<Self, AuthHeaderError>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let header = match req.headers().get(header::AUTHORIZATION) {
            None => return err(AuthHeaderError::Header),
            Some(header) => header,
        };

        match BasicAuth::from_header(header) {
            Ok(auth) => ok(auth),
            Err(e) => err(e),
        }
    }
}
