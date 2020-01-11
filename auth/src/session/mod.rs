mod sessionkey;
mod userid;

use actix_session::{CookieSession, Session};

pub use self::sessionkey::*;
pub use self::userid::*;

pub struct IdentityCookie;

impl IdentityCookie {
    pub fn middleware(key: &[u8]) -> CookieSession {
        CookieSession::signed(key)
            .name("shine_user")
            .domain("localhost")
            .http_only(true)
            //.same_site(SameSite::Lax)
            .max_age(100)
        //.domain("shine.com")
        //.secure(true)
    }

    pub fn clear(session: &Session) {
        session.clear()
    }
}
