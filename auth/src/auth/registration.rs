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
    Terms(String),
    Server(String),
}

#[derive(Debug, Deserialize)]
pub struct RegistrationParams {
    user_name: String,
    email: String,
    password: String,
    accept_terms: Option<bool>,
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
    let name = ValidatedName::from_raw(&params.user_name)
        .map_err(|err| {
            use RegistrationError::*;
            match err {
                NameValidationError::TooShort => errors.push(Username(format!("too_short"))),
                NameValidationError::TooLong => errors.push(Username(format!("too_long"))),
                NameValidationError::InvalidCharacter => errors.push(Username(format!("invalid_character"))),
            }
        })
        .ok();

    let email = if !params.email.is_empty() {
        ValidatedEmail::from_raw(&params.email)
            .map_err(|err| {
                use RegistrationError::*;
                match err {
                    EmailValidationError::InvalidFormat => errors.push(Email(format!("invalid_format"))),
                    EmailValidationError::UnsupportedDomain(_) => errors.push(Email(format!("invalid_domain"))),
                }
            })
            .ok()
    } else {
        None
    };

    let password = ValidatedPassword::from_raw(&params.password)
        .map_err(|err| {
            use RegistrationError::*;
            match err {
                PasswordValidationError::TooShort => errors.push(Password(format!("too_short"))),
                PasswordValidationError::TooLong => errors.push(Password(format!("too_long"))),
                PasswordValidationError::TooWeek => errors.push(Password(format!("too_week"))),
            }
        })
        .ok();

    if !params.accept_terms.unwrap_or(false) {
        errors.push(RegistrationError::Terms(format!("missing")));
    }

    if let Err(err) = state.recaptcha().check_response(&params.recaptcha_response).await {
        errors.push(RegistrationError::Recaptcha(format!("{:?}", err)));
    }

    if errors.is_empty() {
        Ok((name.unwrap(), email, password.unwrap()))
    } else {
        Err(errors)
    }
}

fn gen_page(tera: &Tera, keys: &Keys, params: Option<(RegistrationParams, Vec<RegistrationError>)>) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("user_name_min_len", &format!("{}", ValidatedName::MIN_LEN));
    context.insert("user_name_max_len", &format!("{}", ValidatedName::MAX_LEN));
    context.insert("password_min_len", &format!("{}", ValidatedPassword::MIN_LEN));
    context.insert("password_max_len", &format!("{}", ValidatedPassword::MAX_LEN));

    context.insert("user_name", "");
    context.insert("email", "");
    context.insert("password", "");
    context.insert("af_token", &keys.af);
    context.insert("recaptcha_site_key", &keys.recaptcha_site_key);

    if let Some((params, errors)) = params {
        context.insert("user_name", params.user_name.as_str());
        context.insert("email", params.email.as_str());
        context.insert("password", params.password.as_str());

        context.insert("user_name_validity", "accepted");
        context.insert("email_validity", "accepted");
        context.insert("password_validity", "accepted");
        context.insert("recaptcha_validity", "accepted");
        context.insert("server_validity", "");
        context.insert("terms_validity", "accepted");

        log::info!("page errors: {:?}", errors);
        for err in errors {
            match err {
                RegistrationError::Username(ref err) => context.insert("user_name_validity", err),
                RegistrationError::Email(ref err) => context.insert("email_validity", err),
                RegistrationError::Password(ref err) => context.insert("password_validity", err),
                RegistrationError::Server(ref err) => context.insert("server_validity", err),
                RegistrationError::Recaptcha(ref err) => context.insert("recaptcha_validity", err),
                RegistrationError::Terms(ref err) => context.insert("terms_validity", err),
            };
        }
    } else {
        context.insert("user_name_validity", "");
        context.insert("email_validity", "");
        context.insert("password_validity", "");
        context.insert("recaptcha_validity", "");
        context.insert("terms_validity", "");
        context.insert("server_validity", "");
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
    gen_page(&*state.tera(), &keys, None)
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
        Err(errors) => return gen_page(&*state.tera(), &keys, Some((params, errors))),
        Ok(validated_input) => validated_input,
    };

    // register user
    let (identity, roles, session) = match state.iam().register_user(name, email, password, &fingerprint).await {
        Err(err) => {
            log::info!("user registeration failed: {:?}", err);
            let errors = match err {
                IAMError::NameTaken => vec![RegistrationError::Username("already_taken".to_owned())],
                IAMError::EmailTaken => vec![RegistrationError::Email("already_taken".to_owned())],
                err => vec![RegistrationError::Server(format!("server_error:{:?}", err))],
            };
            return gen_page(&*state.tera(), &keys, Some((params, errors)));
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
