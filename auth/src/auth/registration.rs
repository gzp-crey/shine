use super::iam::{
    identity::{
        EmailValidationError, NameValidationError, PasswordValidationError, ValidatedEmail, ValidatedName,
        ValidatedPassword,
    },
    IAMError,
};
use super::utils::create_user_id;
use super::State;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use shine_core::{
    kernel::{
        anti_forgery::{AntiForgeryIdentity, AntiForgeryIssuer, AntiForgerySession, AntiForgeryValidator},
        identity::{IdentityCookie, IdentitySession, SessionKey},
        response::{PageError, PageResult},
    },
    requestinfo::RemoteInfo,
};
use tera::Tera;

#[derive(Debug)]
pub enum RegistrationError {
    Username(String),
    Email(String),
    Password(String),
    Recaptcha(String),
    Server(String),
}

#[derive(Debug, Deserialize)]
pub struct RegistrationParams {
    user_name: String,
    email: String,
    password: String,
    af: String,
    #[serde(rename = "g-recaptcha-response")]
    recaptcha_response: String,
}

struct Keys {
    af: String,
    recaptcha_site_key: String,
}

async fn validate_input(
    state: &State,
    params: &RegistrationParams,
) -> Result<(ValidatedName, Option<ValidatedEmail>, ValidatedPassword), Vec<RegistrationError>> {
    let mut errors = Vec::new();

    // validate input
    let name = match ValidatedName::from_raw(&params.name) {
        Err(NameValidationError(msg)) => {
            errors.push(RegistrationError::Username(msg));
            None
        }
        Ok(name) => Some(name),
    };
    let email = if !params.email.is_empty() {
        match ValidatedEmail::from_raw(&params.email) {
            Err(EmailValidationError(msg)) => {
                errors.push(RegistrationError::Email(msg));
                None
            }
            Ok(email) => Some(Some(email)),
        }
    } else {
        Some(None)
    };
    let password = match ValidatedPassword::from_raw(&params.password) {
        Err(PasswordValidationError(msg)) => {
            errors.push(RegistrationError::Password(msg));
            None
        }
        Ok(pass) => Some(pass),
    };

    if let Err(err) = state.recaptcha().check_response(&params.recaptcha_response).await {
        errors.push(RegistrationError::Recaptcha(format!("{:?}", err)));
    }

    if errors.is_empty() {
        Ok((name.unwrap(), email.unwrap(), password.unwrap()))
    } else {
        Err(errors)
    }
}

fn gen_page(
    tera: &Tera,
    keys: &Keys,
    params: Option<&RegistrationParams>,
    errors: Option<Vec<RegistrationError>>,
) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("af_token", &keys.af);
    context.insert("user_name", params.map(|p| p.user_name.as_str()).unwrap_or(""));
    context.insert("email", params.map(|p| p.email.as_str()).unwrap_or(""));
    context.insert("password", params.map(|p| p.password.as_str()).unwrap_or(""));
    context.insert("recaptcha_site_key", &keys.recaptcha_site_key);
    context.insert("user_name_error", "");
    context.insert("email_error", "");
    context.insert("password_error", "");
    context.insert("server_error", "");
    context.insert("recaptcha_error", "");
    if params.is_none() {
        context.insert("user_name_validity", "");
        context.insert("email_validity", "");
        context.insert("password_validity", "");
    } else {
        context.insert("user_name_validity", "is-valid");
        context.insert("email_validity", "is-valid");
        context.insert("password_validity", "is-valid");
    }

    if let Some(errors) = errors {
        log::info!("page errors: {:?}", errors);
        for err in errors {
            match err {
                RegistrationError::Username(ref err) => {
                    context.insert("user_name_validity", "is-invalid");
                    context.insert("user_name_error", err);
                }
                RegistrationError::Email(ref err) => {
                    context.insert("email_validity", "is-invalid");
                    context.insert("email_error", err);
                }
                RegistrationError::Password(ref err) => {
                    context.insert("password_validity", "is-invalid");
                    context.insert("password_error", err);
                }
                RegistrationError::Server(ref err) => context.insert("server_error", err),
                RegistrationError::Recaptcha(ref err) => context.insert("recaptcha_error", err),
            };
        }
    }

    let html = tera.render("register.html", &context).map_err(|err| {
        log::error!("Tera render error: {:?}", err);
        PageError::Internal(format!("Template error"))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn get_register_page(state: web::Data<State>, af_session: AntiForgerySession) -> PageResult {
    log::info!("get_register_page");
    let keys = Keys {
        af: AntiForgeryIssuer::issue(&af_session, None),
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };
    gen_page(&*state.tera(), &keys, None, None)
}

pub async fn post_register_page(
    state: web::Data<State>,
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    registration_params: web::Form<RegistrationParams>,
) -> PageResult {
    let params = registration_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;
    log::info!("post_register_user {:?} {:?}", params, fingerprint);

    let keys = Keys {
        af: AntiForgeryValidator::validate(&af_session, &params.af, AntiForgeryIdentity::Ignore)
            .map_err(|err| PageError::RedirectOnError(format!("AF: {:?}", err), "register.html".to_owned()))?,
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };

    IdentityCookie::clear(&identity_session);

    // validate input
    let (name, email, password) = match validate_input(&*state, &params).await {
        Err(errors) => return gen_page(&*state.tera(), &keys, Some(&params), Some(errors)),
        Ok(validated_input) => validated_input,
    };

    // register user
    let (identity, roles, session) = match state.iam().register_user(name, email, password, &fingerprint).await {
        Err(err) => {
            log::info!("user registeration failed: {:?}", err);
            let errors = match err {
                IAMError::NameTaken => vec![RegistrationError::Username("Already taken".to_owned())],
                IAMError::EmailTaken => vec![RegistrationError::Email("Already taken".to_owned())],
                err => vec![RegistrationError::Server(format!("Server error: {:?}", err))],
            };
            return gen_page(&*state.tera(), &keys, Some(&params), Some(errors));
        }
        Ok(registration) => {
            log::info!("user registered: {:?}", registration);
            registration
        }
    };

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;
    Ok(HttpResponse::Ok().finish())
}
