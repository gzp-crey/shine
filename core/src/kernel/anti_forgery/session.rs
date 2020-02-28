use crate::signed_cookie::{CookieSecurity, Key, Session, SignedCookieOptions};
use chrono::Duration as ChronoDuration;

#[derive(Clone)]
pub struct AntiForgeryConfig {
    pub time_to_live: ChronoDuration,
}

pub type AntiForgerySession = Session<AntiForgeryCookie, AntiForgeryConfig>;

pub struct AntiForgeryCookie {
    security: CookieSecurity,
}

impl AntiForgeryCookie {
    pub fn new(key: &[u8]) -> AntiForgeryCookie {
        let key = Key::from_master(key);
        AntiForgeryCookie {
            security: CookieSecurity::Signed(key),
        }
    }

    pub fn clear(session: &AntiForgerySession) {
        session.clear()
    }
}

impl SignedCookieOptions for AntiForgeryCookie {
    fn name(&self) -> &str {
        "saf"
    }

    fn read_only(&self) -> bool {
        false
    }

    fn security(&self) -> &CookieSecurity {
        &self.security
    }

    fn path(&self) -> &str {
        "/"
    }

    fn secure(&self) -> bool {
        false
    }
}
