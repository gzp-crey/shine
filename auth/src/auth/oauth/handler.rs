use super::authorizer::MyAuthorizer;
use super::issuer::MyIssuer;
use super::regsitrar::MyRegistrar;
use actix::{Actor, Context, Handler};
use oxide_auth::{
    endpoint::{Endpoint, OAuthError, OwnerSolicitor, Scopes, Template},
    primitives::authorizer::Authorizer,
    primitives::issuer::Issuer,
    primitives::registrar::Registrar,
    primitives::scope::Scope,
};
use oxide_auth_actix::{OAuthMessage, OAuthOperation, OAuthRequest, OAuthResponse, WebError};

pub struct AuthState {
    registrar: MyRegistrar,
    authorizer: MyAuthorizer,
    issuer: MyIssuer,
    scopes: Vec<Scope>,
}

impl Endpoint<OAuthRequest> for AuthState {
    type Error = WebError;

    fn registrar(&self) -> Option<&dyn Registrar> {
        Some(&self.registrar)
    }

    fn authorizer_mut(&mut self) -> Option<&mut dyn Authorizer> {
        Some(&mut self.authorizer)
    }

    fn issuer_mut(&mut self) -> Option<&mut dyn Issuer> {
        Some(&mut self.issuer)
    }

    fn owner_solicitor(&mut self) -> Option<&mut dyn OwnerSolicitor<OAuthRequest>> {
        None
    }

    fn scopes(&mut self) -> Option<&mut dyn Scopes<OAuthRequest>> {
        Some(&mut self.scopes)
    }

    fn response(&mut self, _request: &mut OAuthRequest, kind: Template) -> Result<OAuthResponse, Self::Error> {
        log::info!("kind: {:?}", kind);
        Ok(OAuthResponse::ok())
    }

    fn error(&mut self, err: OAuthError) -> Self::Error {
        err.into()
    }

    fn web_error(&mut self, err: WebError) -> Self::Error {
        err
    }
}

impl AuthState {
    pub fn new() -> Self {
        AuthState {
            registrar: MyRegistrar::new(),
            authorizer: MyAuthorizer::new(),
            issuer: MyIssuer::new(),
            scopes: vec!["default-scope".parse().unwrap()],
        }
    }

    fn with_solicitor<S: OwnerSolicitor<OAuthRequest>>(&mut self, solicitor: S) -> AuthStateWithSolicitor<'_, S> {
        AuthStateWithSolicitor { inner: self, solicitor }
    }
}

impl Actor for AuthState {
    type Context = Context<Self>;
}

impl<Op, S> Handler<OAuthMessage<Op, S>> for AuthState
where
    Op: OAuthOperation,
    S: OwnerSolicitor<OAuthRequest>,
{
    type Result = Result<Op::Item, Op::Error>;

    fn handle(&mut self, msg: OAuthMessage<Op, S>, _: &mut Self::Context) -> Self::Result {
        let (op, solicitor) = msg.into_inner();
        op.run(&mut self.with_solicitor(solicitor))
    }
}

pub struct AuthStateWithSolicitor<'a, S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    inner: &'a mut AuthState,
    solicitor: S,
}

impl<'a, S> Endpoint<OAuthRequest> for AuthStateWithSolicitor<'a, S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    type Error = WebError;

    fn registrar(&self) -> Option<&dyn Registrar> {
        self.inner.registrar()
    }

    fn authorizer_mut(&mut self) -> Option<&mut dyn Authorizer> {
        self.inner.authorizer_mut()
    }

    fn issuer_mut(&mut self) -> Option<&mut dyn Issuer> {
        self.inner.issuer_mut()
    }

    fn owner_solicitor(&mut self) -> Option<&mut dyn OwnerSolicitor<OAuthRequest>> {
        Some(&mut self.solicitor)
    }

    fn scopes(&mut self) -> Option<&mut dyn Scopes<OAuthRequest>> {
        self.inner.scopes()
    }

    fn response(&mut self, request: &mut OAuthRequest, kind: Template) -> Result<OAuthResponse, Self::Error> {
        self.inner.response(request, kind)
    }

    fn error(&mut self, err: OAuthError) -> Self::Error {
        self.inner.error(err)
    }

    fn web_error(&mut self, err: WebError) -> Self::Error {
        self.inner.web_error(err)
    }
}
