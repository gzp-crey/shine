mod authorizer;
mod handler;
mod issuer;
mod regsitrar;
mod solicitor;

use self::authorizer::MyAuthorizer;
use self::issuer::MyIssuer;
use self::regsitrar::MyRegistrar;
use self::solicitor::{AuthorizeUser, RequestWithAuthorizedUser, RequestWithUserLogin};
use super::State;
use crate::session::UserId;
use actix::{Actor, Context, Handler};
use actix_session::Session;
use actix_web::{web, HttpRequest};
use futures::compat::Future01CompatExt;
use oxide_auth::{
    endpoint::{Endpoint, OAuthError, OwnerSolicitor, Scopes, Template},
    frontends::simple::endpoint::Vacant,
    primitives::authorizer::Authorizer,
    primitives::issuer::Issuer,
    primitives::registrar::Registrar,
    primitives::scope::Scope,
};
use oxide_auth_actix::{Authorize, OAuthMessage, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};

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

struct AuthStateWithSolicitor<'a, S>
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

pub async fn get_authorization(
    session: Session,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> Result<OAuthResponse, WebError> {
    log::info!("get_authorization");
    let user = UserId::from_session(&session).map_err(|_| WebError::Mailbox)?;
    if let Some(user) = user {
        state
            .auth
            .send(Authorize(oath_req).wrap(RequestWithAuthorizedUser::new(state.tera.clone(), user)))
            .compat()
            .await?
    } else {
        state
            .auth
            .send(Authorize(oath_req).wrap(RequestWithUserLogin::new(state.tera.clone())))
            .compat()
            .await?
    }
}

pub async fn post_authorization(
    session: Session,
    _req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> Result<OAuthResponse, WebError> {
    log::info!("post_authorization");
    let user = UserId::from_session(&session).map_err(|_| WebError::Mailbox)?;
    if let Some(user) = user {
        state
            .auth
            .send(Authorize(oath_req).wrap(AuthorizeUser::new(user)))
            .compat()
            .await?
    } else {
        Err(WebError::Endpoint(OAuthError::DenySilently))
    }
}

pub async fn post_token(oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    log::info!("post_token");
    state.auth.send(Token(oath_req).wrap(Vacant)).compat().await?
}

pub async fn post_refresh(oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    state.auth.send(Refresh(oath_req).wrap(Vacant)).compat().await?
}
