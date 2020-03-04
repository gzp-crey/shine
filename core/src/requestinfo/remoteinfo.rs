use super::RequestInfoError;
use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use futures::future::{ready, Ready};
use std::{
    net::{self, IpAddr},
    str,
};

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    agent: String,
    remote: Option<IpAddr>,
}

impl RemoteInfo {
    /// Returns the ip of the client.
    pub fn remote(&self) -> Option<&IpAddr> {
        self.remote.as_ref()
    }

    /// Returns the agent of the client.
    pub fn agent(&self) -> &str {
        &self.agent
    }

    pub fn parse_request(req: &HttpRequest) -> Result<Self, RequestInfoError> {
        let remote = req
            .connection_info()
            .remote()
            .and_then(|remote| remote.parse::<net::SocketAddr>().ok())
            .map(|remote| remote.ip());
        log::trace!("remote: {:?}", remote);

        let agent = match req.headers().get(header::USER_AGENT) {
            None => "unknown",
            Some(header) => header.to_str()?,
        }
        .to_string();
        log::trace!("agent: {:?}", agent);

        Ok(RemoteInfo {
            remote: remote,
            agent: agent,
        })
    }
}

impl FromRequest for RemoteInfo {
    type Config = ();
    type Error = RequestInfoError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(RemoteInfo::parse_request(req))
    }
}
