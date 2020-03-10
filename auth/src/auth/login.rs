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
pub enum LoginError {
    PasswordOrUsername,
    Recaptcha,
}

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    user: String,
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
    params: &LoginParams,
) -> Result<(Option<ValidatedName>, Option<ValidatedEmail>, ValidatedPassword), Vec<LoginError>> {
    let mut errors = Vec::new();

    // validate input
    let name = ValidatedName::from_raw(&params.user).ok();
    let email = ValidatedEmail::from_raw(&params.user).ok();
    let password = ValidatedPassword::from_raw(&params.password).ok();

    if let Err(_err) = state.recaptcha().check_response(&params.recaptcha_response).await {
        errors.push(LoginError::Recaptcha);
    }

    if errors.is_empty() {
        Ok((name, email, password.unwrap()))
    } else {
        Err(errors)
    }
}

fn gen_page(tera: &Tera, keys: &Keys, _params: Option<(LoginParams, Vec<LoginError>)>) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("user", "");
    context.insert("af_token", &keys.af);
    context.insert("recaptcha_site_key", &keys.recaptcha_site_key);

    context.insert("validity", "");

    let html = tera.render("login.html", &context).map_err(|err| {
        log::error!("Tera render error: {:?}", err);
        PageError::Internal(format!("Template error"))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn get_login_page(state: web::Data<State>, af_session: AntiForgerySession) -> PageResult {
    log::info!("get_login_page");
    let keys = Keys {
        af: AntiForgeryIssuer::issue(&af_session, None),
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };
    gen_page(&*state.tera(), &keys, None)
}

pub async fn post_login_page(
    state: web::Data<State>,
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    login_params: web::Form<LoginParams>,
) -> PageResult {
    let params = login_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;
    log::info!("post_login_page {:?} {:?}", params, fingerprint);

    let keys = Keys {
        af: AntiForgeryValidator::validate(&af_session, &params.af, AntiForgeryIdentity::Ignore)
            .map_err(|err| PageError::RedirectOnError(format!("AF: {:?}", err), "register.html".to_owned()))?,
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };

    IdentityCookie::clear(&identity_session);

    // validate input
    let (_name, _email, _password) = match validate_input(&*state, &params).await {
        Err(errors) => return gen_page(&*state.tera(), &keys, Some((params, errors))),
        Ok(validated_input) => validated_input,
    };

    unimplemented!()
    //create_user_id(identity, roles)?.to_session(&identity_session)?;
    //SessionKey::from(session).to_session(&identity_session)?;
    //Ok(HttpResponse::Ok().finish())
}
