use actix_session::{CookieSession, Session};
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserId {
    id: String,
    name: String,
    roles: Vec<String>,
}

impl UserId {
    pub fn new(id: String, name: String, roles: Vec<String>) -> Self {
        UserId { id, name, roles }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn roles(&self) -> &Vec<String> {
        &self.roles
    }
}

impl UserId {
    pub fn cookie_session(key: &[u8]) -> CookieSession {
        CookieSession::signed(key)
            .name("shine_user")
            .domain("localhost")
            .http_only(true)
            //.same_site(SameSite::Lax)
            .max_age(100)
        //.domain("shine.com")
        //.secure(true)
    }

    pub fn from_session(session: &Session) -> Result<Option<Self>, ActixError> {
        session.get::<UserId>("identity")
    }

    pub fn clear_session(session: &Session) {
        session.clear()
    }

    pub fn to_session(self, session: &Session) -> Result<(), ActixError> {
        session.set("identity", self)
    }
}
