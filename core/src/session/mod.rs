mod sessionkey;
mod userid;

use crate::signed_cookie::{CookieSecurity, Key, SameSite, Session, SignedCookieConfiguration};

pub use self::sessionkey::*;
pub use self::userid::*;

pub type IdentitySession = Session<IdentityCookie>;

pub struct IdentityCookie {
    security: CookieSecurity,
    read_only: bool,
}

impl IdentityCookie {
    pub fn new(key: &[u8], read_only: bool) -> IdentityCookie {
        let key = Key::from_master(key);
        IdentityCookie {
            security: CookieSecurity::Signed(key),
            read_only,
        }
    }

    pub fn clear(session: &IdentitySession) {
        session.clear()
    }
}

impl SignedCookieConfiguration for IdentityCookie {
    fn name() -> &'static str {
        "shine_user"
    }
    fn read_only(&self) -> bool {
        self.read_only
    }
    fn security(&self) -> &CookieSecurity {
        &self.security
    }
    fn path(&self) -> &str {
        "/"
    }
    fn domain(&self) -> &str {
        "localhost"
    }
    fn secure(&self) -> bool {
        false
    }
    fn http_only(&self) -> bool {
        true
    }
    fn same_site(&self) -> Option<SameSite> {
        None
    }
    fn max_age(&self) -> Option<i64> {
        None
    }
}
