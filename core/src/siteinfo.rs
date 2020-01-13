use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest, ResponseError};
use futures::future::{ready, Ready};
use serde::{Deserialize, Serialize};
use std::{fmt, net, str};

/// Possible errors while parsing request for SiteInfo.
#[derive(Debug)]
pub enum Error {
    /// Unable to convert header into the str
    ToStrError(header::ToStrError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ToStrError(e) => write!(f, "{}", e),
        }
    }
}

impl ResponseError for Error {}

impl From<header::ToStrError> for Error {
    fn from(e: header::ToStrError) -> Self {
        Error::ToStrError(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteInfo {
    remote: String,
    agent: String,
}

impl SiteInfo {
    /// Returns the ip of the client.
    pub fn remote(&self) -> &str {
        &self.remote
    }

    /// Returns the agent of the client.
    pub fn agent(&self) -> &str {
        &self.agent
    }

    fn parse_request(req: &HttpRequest) -> Result<Self, Error> {
        let remote = req
            .connection_info()
            .remote()
            .and_then(|remote| remote.parse::<net::SocketAddr>().ok())
            .map(|remote| remote.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        log::info!("remote: {:?}", remote);

        let agent = match req.headers().get(header::USER_AGENT) {
            None => "unknown",
            Some(header) => header.to_str()?,
        }
        .to_string();
        log::info!("agent: {:?}", agent);

        Ok(SiteInfo {
            remote: remote,
            agent: agent,
        })
    }
}

impl FromRequest for SiteInfo {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(SiteInfo::parse_request(req))
    }
}
