mod iam;
mod iam_handler;
mod oauth;

use self::iam::{IAMConfig, IAMError, IAM};
use self::oauth::*;
use actix_rt::SystemRunner;
use actix_web::web;
use data_encoding::{DecodeError, BASE64};
use serde::{Deserialize, Serialize};
use shine_core::{session::IdentityCookie, signed_cookie::SignedCookie};
use std::{fmt, rc::Rc};
use tera::{Error as TeraError, Tera};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub iam: IAMConfig,
    pub cookie_session_secret: String,
}

#[derive(Debug)]
pub enum AuthCreateError {
    ConfigureTera(TeraError),
    ConfigureIAM(IAMError),
    ConfigureDecodeSecret(DecodeError),
}

impl fmt::Display for AuthCreateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthCreateError::ConfigureTera(err) => write!(f, "Error in tera configuration: {:?}", err),
            AuthCreateError::ConfigureIAM(err) => write!(f, "Error in IAM configuration: {:?}", err),
            AuthCreateError::ConfigureDecodeSecret(err) => write!(f, "Error during secret configuration: {:?}", err),
        }
    }
}

struct Inner {
    tera: Tera,
    iam: IAM,
}

#[derive(Clone)]
pub struct State(Rc<Inner>);

impl State {
    pub fn new(tera: Tera, iam: IAM) -> Self {
        Self(Rc::new(Inner { tera: tera, iam: iam }))
    }

    pub fn tera(&self) -> &Tera {
        &self.0.tera
    }

    pub fn iam(&self) -> &IAM {
        &self.0.iam
    }
}

#[derive(Clone)]
pub struct AuthService {
    tera: Tera,
    iam: IAM,
    cookie_session_secret: Vec<u8>,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig) -> Result<AuthService, AuthCreateError> {
        let tera = Tera::new("tera_web/**/*").map_err(|err| AuthCreateError::ConfigureTera(err.into()))?;

        let iam_config = config.iam.clone();
        let iam = sys
            .block_on(IAM::new(iam_config))
            .map_err(|err| AuthCreateError::ConfigureIAM(err.into()))?;
        let cookie_session_secret = BASE64
            .decode(config.cookie_session_secret.as_bytes())
            .map_err(|err| AuthCreateError::ConfigureDecodeSecret(err.into()))?;

        Ok(AuthService {
            iam,
            tera,
            cookie_session_secret,
        })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(self.tera.clone(), self.iam.clone());

        services.service(
            web::scope("auth/api")
                .wrap(SignedCookie::new().add(IdentityCookie::write(&self.cookie_session_secret)))
                .data(state)                
                .service(
                    // user authentication
                    web::scope("users")
                        .service(web::resource("login").route(web::post().to(iam_handler::login_basic_auth)))
                        .service(web::resource("register").route(web::post().to(iam_handler::register_user)))
                        .service(web::resource("refresh").route(web::post().to(iam_handler::refresh_session)))
                        .service(web::resource("validate").route(web::post().to(iam_handler::validate_session)))
                        .service(web::resource("refresh_key").route(web::post().to(iam_handler::refresh_session_by_key)))
                        .service(web::resource("logout").route(web::post().to(iam_handler::logout))),
                )
                .service(
                    // role management
                    web::scope("roles")
                        .service(web::resource("").route(web::get().to(iam_handler::get_roles)))
                        /*.service(web::resource("/{role}").route(web::put().to(iam_handler::create_role)))
                        .service(web::resource("/{role}").route(web::post().to(iam_handler::update_role)))
                        .service(web::resource("/{role}").route(web::post().to(iam_handler::delete_role)))*/
                ),
        );
    }
}
