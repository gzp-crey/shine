use super::State;
use oxide_auth::{
    endpoint::PreGrant,
    primitives::prelude::ClientUrl,
    primitives::prelude::{Client, ClientMap},
    primitives::registrar::{BoundClient, Registrar, RegistrarError},
    primitives::scope::Scope,
};
use std::rc::Rc;

pub struct OAuthRegistrar {
    state: Rc<State>,
    registrar: ClientMap,
}

impl OAuthRegistrar {
    pub fn new(state: Rc<State>) -> Self {
        let registrar = vec![Client::public(
            "LocalClient",
            "http://localhost:8021/endpoint".parse().unwrap(),
            "default-scope".parse().unwrap(),
        )]
        .into_iter()
        .collect();

        OAuthRegistrar { state, registrar }
    }
}

impl Registrar for OAuthRegistrar {
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
