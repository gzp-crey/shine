mod identity;
mod oauth;

use self::identity::*;
use self::oauth::*;
use actix_rt::SystemRunner;
use actix_web::web;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use tera::{Error as TeraError, Tera};

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

struct Inner {
    tera: Tera,
    identity_db: IdentityManager,
}

#[derive(Clone)]
pub struct State(Rc<Inner>);

impl State {
    pub fn new(tera: Tera, identity_db: IdentityManager) -> Self {
        Self(Rc::new(Inner {
            tera: tera,
            identity_db: identity_db,
        }))
    }

    pub fn tera(&self) -> &Tera {
        &self.0.tera
    }

    pub fn identity_db(&self) -> &IdentityManager {
        &self.0.identity_db
    }
}

#[derive(Clone)]
pub struct AuthService {
    tera: Tera,
    identity_db: IdentityManager,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig) -> Result<AuthService, AuthError> {
        let tera = Tera::new("tera_web/**/*")?;

        let identity_cfg = config.identity.clone();
        let identity_db = sys.block_on(IdentityManager::new(identity_cfg))?;
        Ok(AuthService { identity_db, tera })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(self.tera.clone(), self.identity_db.clone());

        services.service(
            web::scope("auth/api")
                .data(state)
                .service(
                    // oath2 client (app) authentication
                    web::scope("client")
                        .service(
                            web::resource("authorize")
                                .route(web::get().to(get_authorization))
                                .route(web::post().to(post_authorization)),
                        )
                        .service(web::resource("refresh").route(web::post().to(post_refresh)))
                        .service(web::resource("token").route(web::post().to(post_token))),
                )
                .service(
                    // user authentication
                    web::scope("user")
                        .service(web::resource("login").route(web::post().to(login_basicauth)))
                        .service(web::resource("register").route(web::post().to(register_user)))
                        .service(web::resource("refresh").route(web::post().to(refresh_session))),
                ),
        );
    }
}
