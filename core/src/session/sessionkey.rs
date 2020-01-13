use actix_session::Session;
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SessionKey {
    key: String,
    //expiration: String,
}

impl SessionKey {
    pub fn new(key: String) -> Self {
        SessionKey { key }
    }

    pub fn key(&self) -> &String {
        &self.key
    }
}

impl SessionKey {
    pub fn from_session(session: &Session) -> Result<Option<Self>, ActixError> {
        session.get::<SessionKey>("key")
    }

    pub fn to_session(self, session: &Session) -> Result<(), ActixError> {
        session.set("key", self)
    }
}
