mod oauth;
mod user;

use self::oauth::*;
use self::user::*;
use actix::{Actor, Addr};
use actix_web::web;
use std::sync::Arc;
use tera::Tera;

pub struct State {
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
            .service(web::resource("refresh").route(web::post().to_async(|a, b| post_refresh(a, b).boxed_local().compat())))
            .service(web::resource("token").route(web::post().to_async(|a, b| post_token(a, b).boxed_local().compat())))
            .service(web::resource("login").route(web::post().to_async(|a, b, c| login(a, b, c).boxed_local().compat())))
            .service(web::resource("register").route(web::post().to_async(|a, b, c| register(a, b, c).boxed_local().compat()))),
    );
}
