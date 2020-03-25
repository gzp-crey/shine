use super::iam::{
    identity::{
        EmailValidationError, NameValidationError, PasswordValidationError, ValidatedEmail, ValidatedName,
        ValidatedPassword,
    },
    IAMError,
};
use super::utils::create_user_id;
use super::{State, DEFAULT_PAGE};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use shine_core::{
    kernel::{
        anti_forgery::{AntiForgeryIdentity, AntiForgeryIssuer, AntiForgerySession, AntiForgeryValidator},
        identity::{IdentityCookie, IdentitySession, SessionKey},
        response::{PageError, PageResult, Redirect},
    },
    requestinfo::RemoteInfo,
};
use tera::Tera;

#[derive(Debug)]
pub enum RegistrationError {
    UsernameTooShort,
    UsernameTooLong,
    UsernameInvalid(Vec<char>),
    UsernameAlreadyTaken,
    EmailInvalid,
    EmailInvalidDomain,
    EmailAlreadyTaken,
    PasswordTooShort,
    PasswordTooLong,
    PasswordTooWeek,
    Recaptcha,
    TermsMissing,
    Server(String),
}

#[derive(Debug, Deserialize)]
pub struct RegistrationParams {
    user: String,
    email: String,
    password: String,
    accept_terms: Option<bool>,
    af: String,
    #[serde(rename = "g-recaptcha-response")]
    recaptcha_response: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRedirect {
    redirect: Option<String>,
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
    let name = ValidatedName::from_raw(&params.user)
        .map_err(|err| {
            use RegistrationError::*;
            match err {
                NameValidationError::TooShort => errors.push(UsernameTooShort),
                NameValidationError::TooLong => errors.push(UsernameTooLong),
                NameValidationError::InvalidCharacter(ref err) => errors.push(UsernameInvalid(err.clone())),
            }
        })
        .ok();

    let email = if !params.email.is_empty() {
        ValidatedEmail::from_raw(&params.email)
            .map_err(|err| {
                use RegistrationError::*;
                match err {
                    EmailValidationError::InvalidFormat => errors.push(EmailInvalid),
                    EmailValidationError::UnsupportedDomain(_) => errors.push(EmailInvalidDomain),
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
                PasswordValidationError::TooShort => errors.push(PasswordTooShort),
                PasswordValidationError::TooLong => errors.push(PasswordTooLong),
                PasswordValidationError::TooWeek => errors.push(PasswordTooWeek),
            }
        })
        .ok();

    if !params.accept_terms.unwrap_or(false) {
        errors.push(RegistrationError::TermsMissing);
    }

    if let Err(_err) = state.recaptcha().check_response(&params.recaptcha_response).await {
        errors.push(RegistrationError::Recaptcha);
    }

    if errors.is_empty() {
        Ok((name.unwrap(), email, password.unwrap()))
    } else {
        Err(errors)
    }
}

fn gen_page(
    web_root: &str,
    tera: &Tera,
    lang: &str,
    keys: &Keys,
    redirect: &RegisterRedirect,
    params: Option<(RegistrationParams, Vec<RegistrationError>)>,
) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("root", &format!("/{}", web_root));
    context.insert("lang", lang);

    context.insert("user_min_len", &format!("{}", ValidatedName::MIN_LEN));
    context.insert("user_max_len", &format!("{}", ValidatedName::MAX_LEN));
    context.insert("password_min_len", &format!("{}", ValidatedPassword::MIN_LEN));
    context.insert("password_max_len", &format!("{}", ValidatedPassword::MAX_LEN));

    context.insert("user", "");
    context.insert("email", "");
    context.insert("password", "");
    context.insert("af_token", &keys.af);
    context.insert("recaptcha_site_key", &keys.recaptcha_site_key);

    if let Some(ref redirect) = redirect.redirect {
        context.insert("redirect", &redirect);
    }

    context.insert("user_validity", "");
    context.insert("email_validity", "");
    context.insert("password_validity", "");
    context.insert("recaptcha_validity", "");
    context.insert("terms_validity", "");
    context.insert("server_validity", "");

    if let Some((params, errors)) = params {
        context.insert("user", params.user.as_str());
        context.insert("email", params.email.as_str());
        context.insert("password", params.password.as_str());

        context.insert("user_validity", "accepted");
        context.insert("email_validity", "accepted");
        context.insert("password_validity", "accepted");
        context.insert("recaptcha_validity", "accepted");
        context.insert("terms_validity", "accepted");

        log::info!("page errors: {:?}", errors);
        for err in errors {
            match err {
                RegistrationError::UsernameTooLong => context.insert("user_validity", "err:too_long"),
                RegistrationError::UsernameTooShort => context.insert("user_validity", "err:too_short"),
                RegistrationError::UsernameInvalid(_err) => context.insert("user_validity", "err:invalid"),
                RegistrationError::UsernameAlreadyTaken => context.insert("user_validity", "err:already_taken"),
                RegistrationError::EmailInvalid => context.insert("email_validity", "err:invalid"),
                RegistrationError::EmailInvalidDomain => context.insert("email_validity", "err:invalid_domain"),
                RegistrationError::EmailAlreadyTaken => context.insert("email_validity", "err:already_taken"),
                RegistrationError::PasswordTooLong => context.insert("password_validity", "err:too_long"),
                RegistrationError::PasswordTooShort => context.insert("password_validity", "err:too_short"),
                RegistrationError::PasswordTooWeek => context.insert("password_validity", "err:too_week"),
                RegistrationError::Recaptcha => context.insert("recaptcha_validity", "err:missing"),
                RegistrationError::TermsMissing => context.insert("terms_validity", "err:missing"),
                RegistrationError::Server(ref err) => context.insert("server_validity", &format!("err:{}", err)),
            };
        }
    }

    let html = tera.render("register.html", &context).map_err(|err| {
        log::error!("Tera render error: {:?}", err);
        PageError::Internal(format!("Template error"))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn get_register_page(
    state: web::Data<State>,
    af_session: AntiForgerySession,
    lang: web::Path<String>,
    redirect: web::Query<RegisterRedirect>,
) -> PageResult {
    log::info!("get_register_page");
    let keys = Keys {
        af: AntiForgeryIssuer::issue(&af_session, None),
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };
    gen_page(state.web_root(), &*state.tera(), &*lang, &keys, &*redirect, None)
}

pub async fn post_register_page(
    state: web::Data<State>,
    req: HttpRequest,
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    lang: web::Path<String>,
    redirect: web::Query<RegisterRedirect>,
    registration_params: web::Form<RegistrationParams>,
) -> PageResult {
    let params = registration_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;
    log::info!("post_register_user {:?} {:?}", params, fingerprint);

    let keys = Keys {
        af: AntiForgeryValidator::validate(&af_session, &params.af, AntiForgeryIdentity::Ignore).map_err(|err| {
            let uri = format!("register.html?{}", req.query_string());
            PageError::RedirectOnError(format!("AF error: {:?}", err), Redirect::SeeOther(uri))
        })?,
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };

    IdentityCookie::clear(&identity_session);

    // validate input
    let (name, email, password) = match validate_input(&*state, &params).await {
        Err(errors) => {
            return gen_page(
                state.web_root(),
                &*state.tera(),
                &*lang,
                &keys,
                &*redirect,
                Some((params, errors)),
            )
        }
        Ok(validated_input) => validated_input,
    };

    // register user
    let (identity, roles, session) = match state.iam().register_user(name, email, password, &fingerprint).await {
        Err(err) => {
            log::info!("user registeration failed: {:?}", err);
            let errors = match err {
                IAMError::NameTaken => vec![RegistrationError::UsernameAlreadyTaken],
                IAMError::EmailTaken => vec![RegistrationError::EmailAlreadyTaken],
                err => vec![RegistrationError::Server(format!("server_error:{:?}", err))],
            };
            return gen_page(
                state.web_root(),
                &*state.tera(),
                &*lang,
                &keys,
                &*redirect,
                Some((params, errors)),
            );
        }
        Ok(registration) => {
            log::info!("user registered: {:?}", registration);
            registration
        }
    };

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(Redirect::SeeOther(redirect.redirect.clone().unwrap_or(DEFAULT_PAGE.to_owned())).into())
}
