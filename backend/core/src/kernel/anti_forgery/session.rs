use crate::signed_cookie::{CookieSecurity, Key, Session, SignedCookieOptions};

pub type AntiForgerySession = Session<AntiForgeryCookie, ()>;

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
