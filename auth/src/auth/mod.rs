mod authorizer;
mod handler;
mod issuer;
mod regsitrar;
mod solicitor;

use crate::usersession::UserId;
use actix::{Actor, Addr, MailboxError};
use actix_session::Session;
use actix_web::{web, HttpRequest};
use futures::compat::{Compat, Compat01As03, Future01CompatExt};
use futures::{Future, TryFuture};
use handler::AuthState;
use oxide_auth::frontends::simple::endpoint::Vacant;
use oxide_auth_actix::{Authorize, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};
use solicitor::{AuthorizeWithLogin, RequestAuthorizeWithLogin};
use std::sync::Arc;
use tera::Tera;

struct State {
    auth: Addr<AuthState>,
    tera: Arc<Tera>,
}

impl State {
    fn new() -> Result<State, String> {
        let tera = match Tera::new("tera_web/**/*") {
            Ok(t) => t,
            Err(e) => return Err(format!("Tera template parsing error(s): {}", e)),
        };

        Ok(State {
            auth: AuthState::new().start(),
            tera: Arc::new(tera),
        })
    }
}

async fn get_authorization(session: Session, oath_req: OAuthRequest, state: web::Data<State>) -> Result<OAuthResponse, WebError> {
    log::info!("get_authorization");
    let user = UserId::from_session(&session).map_err(|_| WebError::Mailbox)?;
    if let Some(user) = user {
        log::info!("hi {}", user.name);
    }

    state
        .auth
        .send(Authorize(oath_req).wrap(RequestAuthorizeWithLogin::new(state.tera.clone())))
        .compat()
        .await?
}

fn post_authorization(
    _req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_authorization");
    Compat01As03::new(
        state
            .auth
            .send(Authorize(oath_req).wrap(AuthorizeWithLogin::new(state.tera.clone()))),
    )
}

fn post_token(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_token");
    Compat01As03::new(state.auth.send(Token(oath_req).wrap(Vacant)))
}

fn post_refresh(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    Compat01As03::new(state.auth.send(Refresh(oath_req).wrap(Vacant)))
}

pub fn configure_service(cfg: &mut web::ServiceConfig) {
    use futures::future::{FutureExt, TryFutureExt};
    let data = web::Data::new(State::new().unwrap());
    cfg.service(
        web::scope("auth/api")
            .register_data(data.clone())
            .service(
                web::resource("authorize")
                    .route(web::get().to_async(|a, b, c| get_authorization(a, b, c).boxed_local().compat()))
                    .route(web::post().to_async(|a, b, c| Compat::new(post_authorization(a, b, c)))),
            )
            .service(web::resource("refresh").route(web::post().to_async(|a, b| Compat::new(post_refresh(a, b)))))
            .service(web::resource("token").route(web::post().to_async(|a, b| Compat::new(post_token(a, b))))),
    );
}
