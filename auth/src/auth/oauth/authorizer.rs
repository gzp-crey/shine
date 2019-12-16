use super::State;
use oxide_auth::{primitives::authorizer::Authorizer, primitives::grant::Grant};

impl Authorizer for State {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        log::info!("authorize");
        self.authorizer.authorize(grant)
    }

    fn extract(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        log::info!("extract");
        self.authorizer.extract(token)
    }
}
