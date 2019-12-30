use super::State;
use oxide_auth::{
    primitives::authorizer::Authorizer,
    primitives::grant::Grant,
    primitives::prelude::{AuthMap, RandomGenerator},
};

pub struct OAuthAuthorizer {
    state: State,
    authorizer: AuthMap<RandomGenerator>,
}

impl OAuthAuthorizer {
    pub fn new(state: State) -> Self {
        let authorizer = AuthMap::new(RandomGenerator::new(16));
        OAuthAuthorizer { state, authorizer }
    }
}

impl Authorizer for OAuthAuthorizer {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        log::info!("authorize");
        self.authorizer.authorize(grant)
    }

    fn extract(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        log::info!("extract");
        self.authorizer.extract(token)
    }
}
