use super::State;
use oxide_auth::{
    primitives::grant::Grant,
    primitives::issuer::{IssuedToken, Issuer},
};

impl Issuer for State {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        log::info!("issue");
        self.issuer.issue(grant)
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
