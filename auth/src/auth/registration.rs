use super::iam::IAMError;
use super::utils::create_user_id;
use super::State;
use actix_web::HttpRequest;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use shine_core::kernel::anti_forgery::{AntiForgeryIdentity, AntiForgeryIssuer, AntiForgerySession, AntiForgeryValidator};
use shine_core::kernel::identity::{IdentityCookie, IdentitySession, SessionKey};
use shine_core::kernel::response::{PageError, PageResult};
use tera::Tera;

#[derive(Debug, Deserialize)]
pub struct RegistrationParams {
    name: String,
    email: String,
    password: String,
    af: String,
}

fn gen_page(tera: &Tera, af: &str, params: Option<&RegistrationParams>, err: Option<String>) -> PageResult {
    let mut context = tera::Context::new();
    context.insert("af_token", af);
    context.insert("name", params.map(|p| p.name.as_str()).unwrap_or(""));
    context.insert("email", params.map(|p| p.email.as_str()).unwrap_or(""));
    context.insert("password", params.map(|p| p.password.as_str()).unwrap_or(""));

    let html = tera.render("register.html", &context).map_err(|err| {
        log::error!("Tera render error: {:?}", err);
        PageError::Internal(format!("Template error"))
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn get_register_page(state: web::Data<State>, af_session: AntiForgerySession) -> PageResult {
    log::info!("get_register_page");
    let af_issuer = AntiForgeryIssuer::new(&af_session, None);

    gen_page(state.tera(), af_issuer.token(), None, None)
}

pub async fn post_register_page(
    state: web::Data<State>,
    req: HttpRequest,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    registration_params: web::Form<RegistrationParams>,
) -> PageResult {
    let params = registration_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&req).await?;
    log::info!("post_register_user {:?} {:?}", params, fingerprint);

    let af_validator = AntiForgeryValidator::new(&af_session, AntiForgeryIdentity::Ignore)
        .map_err(|_| PageError::RedirectTo("register.html".to_owned()))?;
    let token = af_validator
        .validate(&params.af)
        .map_err(|_| PageError::RedirectTo("register.html".to_owned()))?;

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = match state
        .iam()
        .register_user(&params.name, params.email.as_deref(), &params.password, &fingerprint)
        .await
    {
        Ok(ok) => ok,
        Err(IAMError::NameInvalid(err)) => return gen_page(state.tera(), &token, Some(&params), Some(err)),
        _ => return Err(PageError::RedirectTo("register.html".to_owned())),
    };

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}
