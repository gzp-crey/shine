mod identity;
mod oauth;

use self::identity::*;
use self::oauth::*;
use actix_rt::SystemRunner;
use actix_web::web;
use data_encoding::{DecodeError, BASE64};
use serde::{Deserialize, Serialize};
use shine_core::{session::IdentityCookie, signed_cookie::SignedCookie};
use std::{fmt, rc::Rc};
use tera::{Error as TeraError, Tera};

pub use self::identity::{IdentityConfig, IdentityError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub identity: IdentityConfig,
    pub cookie_session_secret: String,
}

#[derive(Debug)]
pub enum AuthCreateError {
    ConfigureTera(TeraError),
    ConfigureIdentity(IdentityError),
    ConfigureDecodeSecret(DecodeError),
}

impl fmt::Display for AuthCreateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthCreateError::ConfigureTera(err) => write!(f, "Error in tera configuration: {:?}", err),
            AuthCreateError::ConfigureIdentity(err) => write!(f, "Error in identity configuration: {:?}", err),
            AuthCreateError::ConfigureDecodeSecret(err) => write!(f, "Error during secret configuration: {:?}", err),
        }
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
    cookie_session_secret: Vec<u8>,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig) -> Result<AuthService, AuthCreateError> {
        let tera = Tera::new("tera_web/**/*").map_err(|err| AuthCreateError::ConfigureTera(err.into()))?;

        let identity_cfg = config.identity.clone();
        let identity_db = sys
            .block_on(IdentityManager::new(identity_cfg))
            .map_err(|err| AuthCreateError::ConfigureIdentity(err.into()))?;
        let cookie_session_secret = BASE64
            .decode(config.cookie_session_secret.as_bytes())
            .map_err(|err| AuthCreateError::ConfigureDecodeSecret(err.into()))?;

        Ok(AuthService {
            identity_db,
            tera,
            cookie_session_secret,
        })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(self.tera.clone(), self.identity_db.clone());

        services.service(
            web::scope("auth/api")
                .wrap(SignedCookie::new().add(IdentityCookie::write(&self.cookie_session_secret)))
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
                        .service(web::resource("login").route(web::post().to(login_basic_auth)))
                        .service(web::resource("register").route(web::post().to(register_user)))
                        .service(web::resource("refresh").route(web::post().to(refresh_session)))
                        .service(web::resource("logout").route(web::post().to(logout))),
                ),
        );
    }
}
