use super::RequestInfoError;
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::future::{ready, Ready};
use std::str;

#[derive(Debug, Clone)]
pub struct TestingToken {
    token: Option<String>,
}

impl TestingToken {
    pub fn is_valid(&self) -> bool {
        self.token.is_some()
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn parse_request(req: &HttpRequest) -> Result<Self, RequestInfoError> {
        let token = if let Some(token) = req.headers().get("x-sh-testing-token") {
            let token = token.to_str()?.to_string();
            log::info!("test token: {:?}", token);
            Some(token)
        } else {
            None
        };
        Ok(TestingToken { token })
    }
}

impl FromRequest for TestingToken {
    type Config = ();
    type Error = RequestInfoError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(TestingToken::parse_request(req))
    }
}
