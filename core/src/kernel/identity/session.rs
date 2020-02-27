use crate::signed_cookie::{CookieSecurity, Key, Session, SignedCookieOptions};

pub type IdentitySession = Session<IdentityCookie, ()>;

pub struct IdentityCookie {
    security: CookieSecurity,
    read_only: bool,
}

impl IdentityCookie {
    pub fn write(key: &[u8]) -> IdentityCookie {
        let key = Key::from_master(key);
        IdentityCookie {
            security: CookieSecurity::Signed(key),
            read_only: false,
        }
    }

    pub fn read(key: &[u8]) -> IdentityCookie {
        let key = Key::from_master(key);
        IdentityCookie {
            security: CookieSecurity::Signed(key),
            read_only: true,
        }
    }

    pub fn clear(session: &IdentitySession) {
        session.clear()
    }
}

impl SignedCookieOptions for IdentityCookie {
    fn name(&self) -> &str {
        "su"
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

    fn secure(&self) -> bool {
        false
    }

    /*fn domain(&self) -> &str {
        "localhost"
    }*/
}
