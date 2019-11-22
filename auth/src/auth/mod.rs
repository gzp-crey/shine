mod authorizer;
mod handler;
mod issuer;
mod regsitrar;
mod solicitor;

use crate::session::UserId;
use actix::{Actor, Addr, MailboxError};
use actix_session::Session;
use actix_web::{web, Error as ActixError, HttpRequest, HttpResponse};
use actix_web_httpauth::extractors::basic::BasicAuth;
use futures::compat::{Compat, Compat01As03, Future01CompatExt};
use futures::TryFuture;
use handler::AuthState;
use oxide_auth::endpoint::OAuthError;
use oxide_auth::frontends::simple::endpoint::Vacant;
use oxide_auth_actix::{Authorize, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};
use solicitor::{AuthorizeUser, RequestWithAuthorizedUser, RequestWithUserLogin};
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

async fn post_authorization(
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

fn login(session: Session, auth: BasicAuth, state: web::Data<State>) -> Result<HttpResponse, ActixError> {
    log::info!("login {:?}, {:?}", auth.user_id(), auth.password());
    UserId::new(auth.user_id().to_owned().to_string(), "a".to_string(), vec![]).to_session(&session)?;
    Ok(HttpResponse::Ok().finish())
}

fn index(session: Session, req: HttpRequest) -> actix_web::Result<&'static str> {
    println!("{:?}", req);

    // RequestSession trait is used for session access
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
        session.set("counter", counter)?;
    } else {
        session.set("counter", counter)?;
    }

    Ok("welcome!")
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
                    .route(web::post().to_async(|a, b, c, d| post_authorization(a, b, c, d).boxed_local().compat())),
            )
            .service(web::resource("refresh").route(web::post().to_async(|a, b| Compat::new(post_refresh(a, b)))))
            .service(web::resource("token").route(web::post().to_async(|a, b| Compat::new(post_token(a, b)))))
            .service(web::resource("login").route(web::post().to(login)))
            .service(web::resource("hi").to(index)),
    );
}
