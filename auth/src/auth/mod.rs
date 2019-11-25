mod identity;
mod oauth;
mod state;

use self::identity::*;
use self::oauth::*;
use self::state::State;
use actix::SystemRunner;
use actix_web::web;
use futures::future::{FutureExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tera::Error as TeraError;

pub use self::identity::{IdentityConfig, IdentityError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub identity: IdentityConfig,
}

#[derive(Debug)]
pub enum AuthError {
    Tera(TeraError),
    Identity(IdentityError),
}

impl From<TeraError> for AuthError {
    fn from(err: TeraError) -> Self {
        AuthError::Tera(err)
    }
}

impl From<IdentityError> for AuthError {
    fn from(err: IdentityError) -> Self {
        AuthError::Identity(err)
    }
}

#[derive(Clone)]
pub struct AuthService {
    resources: web::Data<State>,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig) -> Result<AuthService, AuthError> {
        let state = State::new(sys, config)?;
        let resources = web::Data::new(state);
        Ok(AuthService { resources })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        services.service(
            web::scope("auth/api")
                .register_data(self.resources.clone())
                .service(
                    web::resource("authorize")
                        .route(web::get().to_async(|a, b, c| get_authorization(a, b, c).boxed_local().compat()))
                        .route(web::post().to_async(|a, b, c, d| post_authorization(a, b, c, d).boxed_local().compat())),
                )
                .service(web::resource("refresh").route(web::post().to_async(|a, b| post_refresh(a, b).boxed_local().compat())))
                .service(web::resource("token").route(web::post().to_async(|a, b| post_token(a, b).boxed_local().compat())))
                .service(web::resource("login").route(web::post().to_async(|a, b, c| login(a, b, c).boxed_local().compat())))
                .service(
                    web::resource("register").route(web::post().to_async(|a, b, c| register(a, b, c).boxed_local().compat())),
                ),
        );
    }
}
