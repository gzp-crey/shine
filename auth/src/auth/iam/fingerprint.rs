use super::IAMError;
use shine_core::{
    iplocation::{IpLocation, IpLocationProvider},
    requestinfo::RemoteInfo,
};
use std::net::IpAddr;

/// Some very basic fingerprint of the remote to detect
/// session abuse.
#[derive(Clone, Debug)]
pub struct Fingerprint {
    agent: String,
    remote: Option<IpAddr>,
    location: Option<IpLocation>,
    //canvas: Option<String>, - canvas fingerprint generated by js
    //cookie: Option<String>, - local cookie store
}

impl Fingerprint {
    pub async fn new<P: IpLocationProvider>(remote: &RemoteInfo, iplocation: &P) -> Result<Self, IAMError> {
        log::info!("remote: {:?}", remote);

        let location = if let Some(ip) = remote.remote() {
            match iplocation.get_location(ip).await {
                Ok(loc) => Some(loc),
                Err(err) => {
                    log::warn!("Ip not found for {:?}: {:?}", ip, err);
                    None
                }
            }
        } else {
            None
        };
        log::info!("location: {:?}", remote);

        Ok(Fingerprint {
            agent: remote.agent().to_owned(),
            remote: remote.remote().cloned(),
            location: location,
        })
    }

    pub fn agent(&self) -> &str {
        &self.agent
    }

    pub fn remote(&self) -> Option<&IpAddr> {
        self.remote.as_ref()
    }

    pub fn location(&self) -> Option<&IpLocation> {
        self.location.as_ref()
    }
}
