mod authorizer;
mod issuer;
mod registrar;
mod scope;
mod solicitor;

use self::authorizer::*;
use self::issuer::*;
use self::registrar::*;
use self::scope::*;
use self::solicitor::*;
use super::State;
use actix_web::{web, HttpRequest};
use oxide_auth::endpoint::OAuthError;
use oxide_auth_actix::{Authorize, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};
use shine_core::session::{IdentitySession, UserId};

pub async fn get_authorization(
    session: IdentitySession,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> Result<OAuthResponse, WebError> {
    log::info!("get_authorization");
    let user = UserId::from_session(&session).map_err(|err| WebError::InternalError(Some(format!("session: {:?}", err))))?;
    if let Some(user) = user {
        Authorize(oath_req).run(RequestWithAuthorizedUser::solicite(state.get_ref(), user))
    } else {
        Authorize(oath_req).run(RequestWithUserLogin::solicite(state.get_ref()))
    }
}

pub async fn post_authorization(
    session: IdentitySession,
    _req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> Result<OAuthResponse, WebError> {
    log::info!("post_authorization");
    let user = UserId::from_session(&session).map_err(|err| WebError::InternalError(Some(format!("session: {:?}", err))))?;
    if let Some(user) = user {
        Authorize(oath_req).run(AuthorizeUser::solicite(state.get_ref(), user))
    } else {
        Err(WebError::Endpoint(OAuthError::DenySilently))
    }
}

pub async fn post_token(oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    log::info!("post_token");
    Token(oath_req).run(ValidateToken::solicite(state.get_ref()))
}

pub async fn post_refresh(oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    log::info!("post_refresh");
    Refresh(oath_req).run(RefreshToken::solicite(state.get_ref()))
}
