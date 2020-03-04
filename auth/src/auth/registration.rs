use super::iam::IAMError;
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
    name: String,
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

fn gen_page(
    tera: &Tera,
    keys: &Keys,
    params: Option<&RegistrationParams>,
    err: Option<RegistrationError>,
) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("af_token", &keys.af);
    context.insert("name", params.map(|p| p.name.as_str()).unwrap_or(""));
    context.insert("email", params.map(|p| p.email.as_str()).unwrap_or(""));
    context.insert("password", params.map(|p| p.password.as_str()).unwrap_or(""));
    context.insert("recaptcha_site_key", &keys.recaptcha_site_key);

    log::info!("page error: {:?}", err);

    match err {
        None => {}
        Some(RegistrationError::Username(ref err)) => context.insert("name_error", err),
        Some(RegistrationError::Email(ref err)) => context.insert("email_error", err),
        Some(RegistrationError::Password(ref err)) => context.insert("password_error", err),
        Some(RegistrationError::Server(ref err)) => context.insert("server_error", err),
        Some(RegistrationError::Recaptcha(ref err)) => context.insert("recaptcha_error", err),
    };

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
    let email = if params.email.is_empty() {
        None
    } else {
        Some(params.email.as_str())
    };
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;
    log::info!("post_register_user {:?} {:?}", params, fingerprint);

    let keys = Keys {
        af: AntiForgeryValidator::validate(&af_session, &params.af, AntiForgeryIdentity::Ignore)
            .map_err(|err| PageError::RedirectOnError(format!("AF: {:?}", err), "register.html".to_owned()))?,
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };

    if let Err(err) = state.recaptcha().check_response(&params.recaptcha_response).await {
        return gen_page(
            &*state.tera(),
            &keys,
            Some(&params),
            Some(RegistrationError::Recaptcha(format!("{:?}", err))),
        );
    }

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = match state
        .iam()
        .register_user(&params.name, email, &params.password, &fingerprint)
        .await
    {
        Ok(ok) => {
            log::info!("user registered: {:?}", ok);
            ok
        }
        Err(err) => {
            let error = match err {
                IAMError::NameInvalid(err) => RegistrationError::Username(err),
                IAMError::NameTaken => RegistrationError::Username("Already taken".to_owned()),
                IAMError::EmailInvalid(err) => RegistrationError::Email(err),
                IAMError::EmailTaken => RegistrationError::Email("Already taken".to_owned()),
                err => RegistrationError::Server(format!("Server error: {:?}", err)),
            };
            return gen_page(&*state.tera(), &keys, Some(&params), Some(error));
        }
    };

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}
