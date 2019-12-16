use super::State;
use oxide_auth::{
    endpoint::PreGrant,
    primitives::prelude::ClientUrl,
    primitives::registrar::{BoundClient, Registrar, RegistrarError},
    primitives::scope::Scope,
};

impl Registrar for State {
    fn bound_redirect<'a>(&self, bound: ClientUrl<'a>) -> Result<BoundClient<'a>, RegistrarError> {
        log::info!("bound_redirect");
        self.registrar.bound_redirect(bound)
    }

    /// Always overrides the scope with a default scope.
    fn negotiate(&self, bound: BoundClient, scope: Option<Scope>) -> Result<PreGrant, RegistrarError> {
        log::info!("negotiate");
        let res = self.registrar.negotiate(bound, scope);
        log::info!("negotiate: {:?}", res);
        res
    }

    fn check(&self, client_id: &str, passphrase: Option<&[u8]>) -> Result<(), RegistrarError> {
        log::info!("check");
        self.registrar.check(client_id, passphrase)
    }
}
