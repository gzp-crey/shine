use super::IdentitySession;
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionKey {
    key: String,
    //expiration: String,
}

impl SessionKey {
    pub fn new(key: String) -> Self {
        SessionKey { key }
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

impl SessionKey {
    pub fn from_session(session: &IdentitySession) -> Result<Option<Self>, ActixError> {
        session.get::<SessionKey>("se")
    }

    pub fn to_session(self, session: &IdentitySession) -> Result<(), ActixError> {
        session.set("se", self)
    }
}
