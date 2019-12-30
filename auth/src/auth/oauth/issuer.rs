use super::State;
use oxide_auth::{
    primitives::grant::Grant,
    primitives::issuer::{IssuedToken, Issuer, RefreshedToken},
    primitives::prelude::{RandomGenerator, TokenMap},
};

pub struct OAuthIssuer {
    state: State,
    issuer: TokenMap<RandomGenerator>,
}

impl OAuthIssuer {
    pub fn new(state: State) -> Self {
        let issuer = TokenMap::new(RandomGenerator::new(16));
        OAuthIssuer { state, issuer }
    }
}

impl Issuer for OAuthIssuer {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        log::info!("issue");
        self.issuer.issue(grant)
    }

    fn refresh(&mut self, refresh: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        log::info!("recover_token");
        self.issuer.refresh(refresh, grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        log::info!("recover_token");
        self.issuer.recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        log::info!("recover_refresh");
        self.issuer.recover_refresh(token)
    }
}
