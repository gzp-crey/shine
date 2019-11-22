use oxide_auth::{
    primitives::authorizer::Authorizer,
    primitives::grant::Grant,
    primitives::prelude::{AuthMap, RandomGenerator},
};

pub struct MyAuthorizer {
    inner: AuthMap<RandomGenerator>,
}

impl MyAuthorizer {
    pub fn new() -> Self {
        let authorizer = AuthMap::new(RandomGenerator::new(16));
        MyAuthorizer { inner: authorizer }
    }
}

impl Authorizer for MyAuthorizer {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        log::info!("authorize");
        self.inner.authorize(grant)
    }

    fn extract(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        log::info!("extract");
        self.inner.extract(token)
    }
}
