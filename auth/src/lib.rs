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
mod login;
mod registration;
mod trace_middleware;
mod utils;

use self::iam::{IAMConfig, IAMError, IAM};
use self::trace_middleware::Trace;

pub const DEFAULT_PAGE: &str = "google.com";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub iam: IAMConfig,
    pub tera_templates: String,
    pub web_folder: String,
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

struct StateInner {
    web_root: String,
    tera: RefCell<Tera>,
    iam: IAM,
    recaptcha: Recaptcha,
}

#[derive(Clone)]
pub struct State(Rc<StateInner>);

impl State {
    pub fn new(web_root: String, tera: Tera, iam: IAM, recaptcha: Recaptcha) -> Self {
        Self(Rc::new(StateInner {
            web_root,
            tera: RefCell::new(tera),
            iam,
            recaptcha,
        }))
    }

    pub fn web_root(&self) -> &str {
        &self.0.web_root
    }

    pub fn tera(&self) -> Ref<Tera> {
        self.0.tera.borrow()
    }

    pub fn try_reload_tera(&self) -> Result<(), String> {
        self.0
            .tera
            .try_borrow_mut()
            .map_err(|_| "Tera is already in use".to_owned())
            .and_then(|mut tera| {
                tera.full_reload()
                    .map_err(|err| format!("Tera load failed with {:?}", err))
            })
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
    web_folder: String,
    web_root: String,
    id_session_secret: Vec<u8>,
    af_session_secret: Vec<u8>,
}

impl AuthService {
    pub fn create(sys: &mut SystemRunner, config: &AuthConfig, web_root: &str) -> Result<AuthService, AuthCreateError> {
        log::info!("Parsing tera templates");
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
            web_folder: config.web_folder.clone(),
            web_root: web_root.to_owned(),
            id_session_secret,
            af_session_secret,
        })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(
            self.web_root.clone(),
            self.tera.clone(),
            self.iam.clone(),
            self.recaptcha.clone(),
        );

        services.service(
            web::scope(&self.web_root)
                .wrap(Trace::new(state.clone()))
                .wrap(SignedCookie::new(IdentityCookie::write(&self.id_session_secret), ()))
                .wrap(SignedCookie::new(AntiForgeryCookie::new(&self.af_session_secret), ()))
                .data(state)
                .service(actix_files::Files::new("/static", &self.web_folder))
                .service(
                    web::scope("")
                        .service(
                            web::resource("{lang}/register.html")
                                .route(web::get().to(registration::get_register_page))
                                .route(web::post().to(registration::post_register_page)),
                        )
                        .service(
                            web::resource("{lang}/login.html")
                                .route(web::get().to(login::get_login_page))
                                .route(web::post().to(login::post_login_page)),
                        ),
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
