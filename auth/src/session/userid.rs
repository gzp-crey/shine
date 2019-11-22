use actix_session::{CookieSession, Session};
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};

const USER_ID_COOKIE: &str = "shineuser";

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
            .name(USER_ID_COOKIE)
            .domain("localhost")
            .http_only(true)
            //.same_site(SameSite::Lax)
            .max_age(100)
        //.domain("shine.com")
        //.secure(true)
    }

    pub fn from_session(session: &Session) -> Result<Option<Self>, ActixError> {
        session.get::<UserId>(USER_ID_COOKIE)
    }

    pub fn to_session(self, session: &Session) -> Result<(), ActixError> {
        session.set(USER_ID_COOKIE, self)
    }
}
