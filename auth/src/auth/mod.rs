use actix_rt::SystemRunner;
use actix_web::web;
use data_encoding::{DecodeError, BASE64};
use serde::{Deserialize, Serialize};
use shine_core::{
    kernel::{anti_forgery::AntiForgeryCookie, identity::IdentityCookie},
    recaptcha::Recaptcha,
    signed_cookie::SignedCookie,
};
use std::{
    cell::{Ref, RefCell},
    fmt,
    rc::Rc,
};
use tera::{Error as TeraError, Tera};

mod iam;
mod iam_handler;
mod registration;
mod trace_middleware;
mod utils;

use self::iam::{IAMConfig, IAMError, IAM};
use self::trace_middleware::Trace;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub iam: IAMConfig,
    pub tera_templates: String,
    pub recaptcha_secret: String,
    pub recaptcha_site_key: String,
    pub id_session_secret: String,
    pub af_session_secret: String,
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
    tera: RefCell<Tera>,
    iam: IAM,
    recaptcha: Recaptcha,
}

#[derive(Clone)]
pub struct State(Rc<Inner>);

impl State {
    pub fn new(tera: Tera, iam: IAM, recaptcha: Recaptcha) -> Self {
        Self(Rc::new(Inner {
            tera: RefCell::new(tera),
            iam,
            recaptcha,
        }))
    }

    pub fn tera(&self) -> Ref<Tera> {
        self.0.tera.borrow()
    }

    pub fn try_reload_tera(&self) -> Result<(), ()> {
        self.0
            .tera
            .try_borrow_mut()
            .map_err(|_| ())
            .and_then(|mut tera| tera.full_reload().map_err(|_| ()))
    }

    pub fn iam(&self) -> &IAM {
        &self.0.iam
    }

    pub fn recaptcha(&self) -> &Recaptcha {
        &self.0.recaptcha
    }
}

#[derive(Clone)]
pub struct AuthService {
    tera: Tera,
    iam: IAM,
    recaptcha: Recaptcha,
    id_session_secret: Vec<u8>,
    af_session_secret: Vec<u8>,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig) -> Result<AuthService, AuthCreateError> {
        let tera = Tera::new(&config.tera_templates).map_err(|err| AuthCreateError::ConfigureTera(err.into()))?;

        let recaptcha = Recaptcha::new(config.recaptcha_secret.clone(), config.recaptcha_site_key.clone());

        let iam_config = config.iam.clone();
        let iam = sys
            .block_on(IAM::new(iam_config))
            .map_err(|err| AuthCreateError::ConfigureIAM(err.into()))?;
        let id_session_secret = BASE64
            .decode(config.id_session_secret.as_bytes())
            .map_err(|err| AuthCreateError::ConfigureDecodeSecret(err.into()))?;
        let af_session_secret = BASE64
            .decode(config.af_session_secret.as_bytes())
            .map_err(|err| AuthCreateError::ConfigureDecodeSecret(err.into()))?;

        Ok(AuthService {
            iam,
            tera,
            recaptcha,
            id_session_secret,
            af_session_secret,
        })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(self.tera.clone(), self.iam.clone(), self.recaptcha.clone());

        services.service(
            web::scope("auth")
                .wrap(Trace)
                .wrap(SignedCookie::new(IdentityCookie::write(&self.id_session_secret), ()))
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
                                .service(
                                    web::resource("refresh_key")
                                        .route(web::post().to(iam_handler::refresh_session_by_key)),
                                )
                                .service(web::resource("logout").route(web::post().to(iam_handler::logout)))
                                .service(
                                    web::resource("/{user}/roles").route(web::get().to(iam_handler::get_user_roles)),
                                )
                                .service(
                                    web::resource("/{user}/roles/{role}")
                                        .route(web::post().to(iam_handler::add_user_role)),
                                )
                                .service(
                                    web::resource("/{user}/roles/{role}")
                                        .route(web::delete().to(iam_handler::remove_user_role)),
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
