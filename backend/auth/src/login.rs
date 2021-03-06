use super::iam::{
    identity::{ValidatedEmail, ValidatedName, ValidatedPassword},
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
pub enum LoginError {
    Username,
    Password,
    Server(String),
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

#[derive(Debug, Deserialize)]
pub struct LoginRedirect {
    redirect: Option<String>,
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
    let password = ValidatedPassword::from_raw(&params.password)
        .map_err(|_err| errors.push(LoginError::Password))
        .ok();

    if let Err(_err) = state.recaptcha().check_response(&params.recaptcha_response).await {
        errors.push(LoginError::Recaptcha);
    }

    if errors.is_empty() {
        Ok((name, email, password.unwrap()))
    } else {
        Err(errors)
    }
}

fn gen_page(
    web_root: &str,
    tera: &Tera,
    lang: &str,
    keys: &Keys,
    redirect: &LoginRedirect,
    params: Option<(LoginParams, Vec<LoginError>)>,
) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("root", &format!("/{}", web_root));
    context.insert("lang", lang);

    context.insert("user", "");
    context.insert("af_token", &keys.af);
    context.insert("recaptcha_site_key", &keys.recaptcha_site_key);

    if let Some(ref redirect) = redirect.redirect {
        context.insert("redirect", &redirect);
    }

    context.insert("login_validity", "");
    context.insert("recaptcha_validity", "");
    context.insert("server_validity", "");

    if let Some((params, errors)) = params {
        context.insert("user", &params.user);

        log::info!("page errors: {:?}", errors);
        for err in errors {
            match err {
                LoginError::Username => context.insert("login_validity", "err:password_or_name"),
                LoginError::Password => context.insert("login_validity", "err:password_or_name"),
                LoginError::Recaptcha => context.insert("recaptcha_validity", "err:recaptche"),
                LoginError::Server(ref err) => context.insert("server_validity", &format!("err:{}", err)),
            };
        }
    }

    let html = tera.render("login.html", &context).map_err(|err| {
        log::error!("Tera render error: {:?}", err);
        PageError::Internal(format!("Template error"))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn get_login_page(
    state: web::Data<State>,
    af_session: AntiForgerySession,
    lang: web::Path<String>,
    redirect: web::Query<LoginRedirect>,
) -> PageResult {
    log::info!("get_login_page {:?}", redirect);
    let keys = Keys {
        af: AntiForgeryIssuer::issue(&af_session, None),
        recaptcha_site_key: state.recaptcha().site_key().to_owned(),
    };
    gen_page(state.web_root(), &*state.tera(), &*lang, &keys, &*redirect, None)
}

pub async fn post_login_page(
    state: web::Data<State>,
    req: HttpRequest,
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    lang: web::Path<String>,
    redirect: web::Query<LoginRedirect>,
    login_params: web::Form<LoginParams>,
) -> PageResult {
    let params = login_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;
    log::info!("post_login_page {:?} {:?} {:?}", redirect, params, fingerprint);

    let keys = Keys {
        af: AntiForgeryValidator::validate(&af_session, &params.af, AntiForgeryIdentity::Ignore).map_err(|err| {
            let uri = format!("login.html?{}", req.query_string());
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

    let login_result = if let Some(name) = name {
        state.iam().login_by_name(&name, &password, &fingerprint).await
    } else if let Some(email) = email {
        state.iam().login_by_email(&email, &password, &fingerprint).await
    } else {
        Err(IAMError::IdentityNotFound)
    };

    match login_result {
        Ok((identity, roles, session)) => {
            create_user_id(identity, roles)?.to_session(&identity_session)?;
            SessionKey::from(session).to_session(&identity_session)?;
            Ok(Redirect::SeeOther(redirect.redirect.clone().unwrap_or(DEFAULT_PAGE.to_owned())).into())
        }

        Err(err) => {
            log::info!("user login failed: {:?}", err);
            let errors = match err {
                IAMError::IdentityNotFound => vec![LoginError::Username],
                IAMError::PasswordNotMatching => vec![LoginError::Password],
                err => vec![LoginError::Server(format!("server_error:{:?}", err))],
            };
            gen_page(
                state.web_root(),
                &*state.tera(),
                &*lang,
                &keys,
                &*redirect,
                Some((params, errors)),
            )
        }
    }
}
