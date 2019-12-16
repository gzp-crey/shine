mod authorizer;
mod issuer;
mod registrar;

use super::identity::IdentityDB;
use super::{AuthConfig, AuthError};
use crate::session::UserId;
use actix_rt::SystemRunner;
use actix_session::Session;
use actix_web::{web, HttpRequest};
use oxide_auth::{
    endpoint::{Endpoint, OAuthError, OwnerSolicitor, Scopes, Template},
    primitives::authorizer::Authorizer,
    primitives::issuer::Issuer,
    primitives::prelude::{AuthMap, Client, ClientMap, RandomGenerator, TokenMap},
    primitives::registrar::Registrar,
    primitives::scope::Scope,
};
use oxide_auth_actix::{Authorize, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};
use tera::Tera;

pub struct State {
    pub tera: Tera,
    pub identity_db: IdentityDB,

    pub registrar: ClientMap,
    pub authorizer: AuthMap<RandomGenerator>,
    pub issuer: TokenMap<RandomGenerator>,
    pub scopes: Vec<Scope>,
}

impl State {
    pub fn new(tera: Tera, identity_db: IdentityDB) -> State {
        let registrar = vec![Client::public(
            "LocalClient",
            "http://localhost:8021/endpoint".parse().unwrap(),
            "default-scope".parse().unwrap(),
        )]
        .into_iter()
        .collect();

        let authorizer = AuthMap::new(RandomGenerator::new(16));

        let issuer = TokenMap::new(RandomGenerator::new(16));

        State {
            tera,
            identity_db,
            registrar,
            authorizer,
            issuer,
            scopes: vec!["default-scope".parse().unwrap()],
        }
    }

    fn with_solicitor<S: OwnerSolicitor<OAuthRequest>>(&mut self, solicitor: S) -> AuthStateWithSolicitor<'_, S> {
        AuthStateWithSolicitor { inner: self, solicitor }
    }
}

impl Endpoint<OAuthRequest> for State {
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

struct AuthStateWithSolicitor<'a, S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    inner: &'a mut State,
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
    let user = UserId::from_session(&session).map_err(|err| WebError::InternalError(Some(format!("session: {:?}", err))))?;
    if let Some(user) = user {
        unimplemented!()
    /*state
    .auth
    .send(Authorize(oath_req).wrap(RequestWithAuthorizedUser::new(state.tera.clone(), user)))
    .compat()
    .await?*/
    } else {
        unimplemented!()
        /*state
        .auth
        .send(Authorize(oath_req).wrap(RequestWithUserLogin::new(state.tera.clone())))
        .compat()
        .await?*/
    }
}

pub async fn post_authorization(
    session: Session,
    _req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> Result<OAuthResponse, WebError> {
    log::info!("post_authorization");
    let user = UserId::from_session(&session).map_err(|err| WebError::InternalError(Some(format!("session: {:?}", err))))?;
    if let Some(user) = user {
        unimplemented!()
    /*state
    .auth
    .send(Authorize(oath_req).wrap(AuthorizeUser::new(user)))
    .compat()
    .await?*/
    } else {
        Err(WebError::Endpoint(OAuthError::DenySilently))
    }
}

pub async fn post_token(oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    log::info!("post_token");
    unimplemented!()
    //state.auth.send(Token(oath_req).wrap(Vacant)).compat().await?
}

pub async fn post_refresh(oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    unimplemented!()
    //state.auth.send(Refresh(oath_req).wrap(Vacant)).compat().await?
}
