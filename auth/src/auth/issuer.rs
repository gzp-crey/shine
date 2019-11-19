use oxide_auth::{
    primitives::grant::Grant,
    primitives::issuer::{IssuedToken, Issuer},
    primitives::prelude::{RandomGenerator, TokenMap},
};

pub struct MyIssuer {
    inner: TokenMap<RandomGenerator>,
}

impl MyIssuer {
    pub fn new() -> Self {
        let issuer = TokenMap::new(RandomGenerator::new(16));
        MyIssuer { inner: issuer }
    }
}

impl Issuer for MyIssuer {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        log::info!("issue");
        self.inner.issue(grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        log::info!("recover_token");
        self.inner.recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        log::info!("recover_refresh");
        self.inner.recover_refresh(token)
    }
}
