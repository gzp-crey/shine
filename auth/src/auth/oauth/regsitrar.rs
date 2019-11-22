use oxide_auth::{
    endpoint::PreGrant,
    primitives::prelude::{Client, ClientMap, ClientUrl},
    primitives::registrar::{BoundClient, Registrar, RegistrarError},
    primitives::scope::Scope,
};

pub struct MyRegistrar {
    inner: ClientMap,
}

impl MyRegistrar {
    pub fn new() -> Self {
        let registrar = vec![Client::public(
            "LocalClient",
            "http://localhost:8021/endpoint".parse().unwrap(),
            "default-scope".parse().unwrap(),
        )]
        .into_iter()
        .collect();
        MyRegistrar { inner: registrar }
    }
}

impl Registrar for MyRegistrar {
    fn bound_redirect<'a>(&self, bound: ClientUrl<'a>) -> Result<BoundClient<'a>, RegistrarError> {
        log::info!("bound_redirect");
        self.inner.bound_redirect(bound)
    }

    /// Always overrides the scope with a default scope.
    fn negotiate(&self, bound: BoundClient, scope: Option<Scope>) -> Result<PreGrant, RegistrarError> {
        log::info!("negotiate");
        let res = self.inner.negotiate(bound, scope);
        log::info!("negotiate: {:?}", res);
        res
    }

    fn check(&self, client_id: &str, passphrase: Option<&[u8]>) -> Result<(), RegistrarError> {
        log::info!("check");
        self.inner.check(client_id, passphrase)
    }
}
