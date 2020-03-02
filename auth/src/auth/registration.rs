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

pub enum RegistrationError {
    Username(String),
    Email(String),
    Password(String),
    Server(String),
}

#[derive(Debug, Deserialize)]
pub struct RegistrationParams {
    name: String,
    email: String,
    password: String,
    af: String,
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

    let html = tera.render("register.html", &context).map_err(|err| {
        log::error!("Tera render error: {:?}", err);
        PageError::Internal(format!("Template error"))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn get_register_page(state: web::Data<State>, af_session: AntiForgerySession) -> PageResult {
    log::info!("get_register_page");
    let af_issuer = AntiForgeryIssuer::new(&af_session, None);
    let keys = Keys {
        af: af_issuer.token().to_owned(),
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };

    gen_page(state.tera(), &keys, None, None)
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

    let keys = {
        //state.recaptcha().check("")?;
        let af_validator = AntiForgeryValidator::new(&af_session, AntiForgeryIdentity::Ignore)
            .map_err(|_| PageError::RedirectTo("register.html".to_owned()))?;
        let token = af_validator
            .validate(&params.af)
            .map_err(|_| PageError::RedirectTo("register.html".to_owned()))?;
        Keys {
            af: token.to_owned(),
            recaptcha_site_key: state.recaptcha().site_key().to_owned(),
        }
    };

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = match state
        .iam()
        .register_user(&params.name, email, &params.password, &fingerprint)
        .await
    {
        Ok(ok) => ok,
        Err(IAMError::NameInvalid(err)) => {
            return gen_page(
                state.tera(),
                &keys,
                Some(&params),
                Some(RegistrationError::Username(err)),
            )
        }
        Err(IAMError::NameTaken) => {
            return gen_page(
                state.tera(),
                &keys,
                Some(&params),
                Some(RegistrationError::Username("Already taken".to_owned())),
            )
        }
        Err(IAMError::EmailInvalid(err)) => {
            return gen_page(state.tera(), &keys, Some(&params), Some(RegistrationError::Email(err)))
        }
        Err(IAMError::EmailTaken) => {
            return gen_page(
                state.tera(),
                &keys,
                Some(&params),
                Some(RegistrationError::Email("Already taken".to_owned())),
            )
        }
        _ => return Err(PageError::RedirectTo("register.html".to_owned())),
    };

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}
