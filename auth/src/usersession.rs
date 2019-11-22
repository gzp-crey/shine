use actix_session::{CookieSession, Session};
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};

const USER_ID_COOKIE: &str = "shineuser";

#[derive(Serialize, Deserialize)]
pub struct UserId {
    pub id: String,
    pub name: String,
    pub roles: Vec<String>,
}

impl UserId {
    pub fn cookie_session(key: &[u8]) -> CookieSession {
        CookieSession::signed(key).domain("shine.com").secure(true)
    }

    pub fn from_session(session: &Session) -> Result<Option<Self>, ActixError> {
        session.get::<UserId>(USER_ID_COOKIE)
    }

    pub fn to_session(self, session: &Session) -> Result<(), ActixError> {
        session.set(USER_ID_COOKIE, self)
    }
}
