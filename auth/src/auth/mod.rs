mod iam;
mod iam_handler;
mod registration;
mod utils;

use self::iam::{IAMConfig, IAMError, IAM};
use actix_rt::SystemRunner;
use actix_web::web;
use chrono::Duration as ChronoDuration;
use data_encoding::{DecodeError, BASE64};
use serde::{Deserialize, Serialize};
use shine_core::{
    kernel::{anti_forgery::AntiForgeryCookie, identity::IdentityCookie},
    signed_cookie::SignedCookie,
};
use std::{fmt, rc::Rc};
use tera::{Error as TeraError, Tera};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub iam: IAMConfig,
    pub tera_templates: String,
    pub identity_session_secret: String,
    pub af_session_secret: String,
    pub af_time_to_live_m: i16,
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
        Self(Rc::new(Inner { tera, iam }))
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
    identity_session_secret: Vec<u8>,
    af_session_secret: Vec<u8>,
    af_time_to_live: ChronoDuration,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig) -> Result<AuthService, AuthCreateError> {
        let tera = Tera::new(&config.tera_templates).map_err(|err| AuthCreateError::ConfigureTera(err.into()))?;

        let iam_config = config.iam.clone();
        let iam = sys
            .block_on(IAM::new(iam_config))
            .map_err(|err| AuthCreateError::ConfigureIAM(err.into()))?;
        let identity_session_secret = BASE64
            .decode(config.identity_session_secret.as_bytes())
            .map_err(|err| AuthCreateError::ConfigureDecodeSecret(err.into()))?;
        let af_session_secret = BASE64
            .decode(config.af_session_secret.as_bytes())
            .map_err(|err| AuthCreateError::ConfigureDecodeSecret(err.into()))?;

        Ok(AuthService {
            iam,
            tera,
            identity_session_secret,
            af_session_secret,
            af_time_to_live: ChronoDuration::minutes(config.af_time_to_live_m as i64),
        })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(self.tera.clone(), self.iam.clone());

        services.service(
            web::scope("auth")
                .wrap(SignedCookie::new(IdentityCookie::write(&self.identity_session_secret), ()))
                .wrap(SignedCookie::new(AntiForgeryCookie::new(&self.af_session_secret), ()))
                .data(state)
                .service(
                    web::resource("register.html")
                        .route(web::get().to(registration::get_register_page))
                        .route(web::post().to(registration::post_register_page)),
                )
                .service(
                    web::scope("api")
                        .service(web::resource("af").route(web::post().to(iam_handler::create_af_token)))
                        .service(
                            web::scope("users")
                                .service(web::resource("login").route(web::post().to(iam_handler::login_basic_auth)))
                                .service(web::resource("register").route(web::post().to(iam_handler::register_user)))
                                .service(web::resource("refresh").route(web::post().to(iam_handler::refresh_session)))
                                .service(web::resource("validate").route(web::post().to(iam_handler::validate_session)))
                                .service(web::resource("refresh_key").route(web::post().to(iam_handler::refresh_session_by_key)))
                                .service(web::resource("logout").route(web::post().to(iam_handler::logout)))
                                .service(web::resource("/{user}/roles").route(web::get().to(iam_handler::get_user_roles)))
                                .service(web::resource("/{user}/roles/{role}").route(web::post().to(iam_handler::add_user_role)))
                                .service(
                                    web::resource("/{user}/roles/{role}").route(web::delete().to(iam_handler::remove_user_role)),
                                ),
                        )
                        .service(
                            web::scope("roles")
                                .service(web::resource("").route(web::get().to(iam_handler::get_roles)))
                                .service(
                                    web::resource("/{role}")
                                        .route(web::post().to(iam_handler::create_role))
                                        .route(web::delete().to(iam_handler::delete_role)),
                                )
                                .service(
                                    web::resource("/{role}/inherit/{inherited_role}")
                                        .route(web::post().to(iam_handler::inherit_role))
                                        .route(web::delete().to(iam_handler::disherit_role)),
                                ),
                        ),
                ),
        );
    }
}
