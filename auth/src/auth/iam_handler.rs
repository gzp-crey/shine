use super::iam::{
    identity::{Identity, UserIdentity},
    role::Roles,
    IAMError,
};
use super::State;
use actix_web::HttpRequest;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};
use shine_core::kernel::anti_forgery::{AntiForgeryIdentity, AntiForgeryIssuer, AntiForgerySession, AntiForgeryValidator};
use shine_core::kernel::identity::{IdentityCookie, IdentitySession, SessionKey, UserId};

use shine_core::requestinfo::{BasicAuth, TestingToken};

fn create_user_id(user: UserIdentity, roles: Roles) -> Result<UserId, IAMError> {
    let data = user.into_data();
    let user_name = data
        .core
        .name
        .to_raw()
        .map_err(|err| IAMError::Internal(format!("Name decript error: {}", err)))?;
    Ok(UserId::new(data.core.id, user_name, roles))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationParams {
    name: String,
    password: String,
    email: Option<String>,
    af: String,
}

pub async fn register_user(
    state: web::Data<State>,
    req: HttpRequest,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    testing_token: TestingToken,
    registration_params: web::Json<RegistrationParams>,
) -> Result<HttpResponse, ActixError> {
    let RegistrationParams {
        name,
        password,
        email,
        af,
    } = registration_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&req).await?;
    log::info!(
        "register_user[{:?},{:?}]: {}, {}, {:?}",
        testing_token.token(),
        fingerprint,
        name,
        password,
        email
    );

    let af_validator = AntiForgeryValidator::new(&af_session, "register_user".to_owned(), AntiForgeryIdentity::Ignore)?;
    if testing_token.is_valid() {
        state.iam().check_permission_by_testing_token(testing_token.token()).await?;
    } else {
        af_validator.validate(&af).map_err(IAMError::from)?;
    }

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = state
        .iam()
        .register_user(&name, email.as_deref(), &password, &fingerprint)
        .await?;

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn login_basic_auth(
    state: web::Data<State>,
    req: HttpRequest,
    identity_session: IdentitySession,
    auth: BasicAuth,
) -> Result<HttpResponse, ActixError> {
    let user_id = auth.user_id();
    let password = auth.password().ok_or(IAMError::PasswordNotMatching)?;
    let fingerprint = state.iam().get_fingerprint(&req).await?;

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = state.iam().login_name_email(&user_id, password, &fingerprint).await?;

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Deserialize)]
pub struct RefreshKeyParams {
    key: String,
}

pub async fn refresh_session_by_key(
    state: web::Data<State>,
    req: HttpRequest,
    identity_session: IdentitySession,
    key_params: web::Json<RefreshKeyParams>,
) -> Result<HttpResponse, ActixError> {
    let fingerprint = state.iam().get_fingerprint(&req).await?;

    match state.iam().refresh_session_by_key(&key_params.key, &fingerprint).await {
        Ok((identity, roles, session)) => {
            IdentityCookie::clear(&identity_session);
            create_user_id(identity, roles)?.to_session(&identity_session)?;
            SessionKey::from(session).to_session(&identity_session)?;
            Ok(HttpResponse::Ok().finish())
        }
        Err(e @ IAMError::SessionKeyConflict) => {
            // Preserve cookie and report a conflict error
            Err(e.into())
        }
        Err(e) => {
            IdentityCookie::clear(&identity_session);
            Err(e.into())
        }
    }
}

pub async fn validate_session(
    state: web::Data<State>,
    req: HttpRequest,
    identity_session: IdentitySession,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let fingerprint = state.iam().get_fingerprint(&req).await?;

    match state
        .iam()
        .validate_session(user_id.user_id(), session_key.key(), &fingerprint)
        .await
    {
        Ok((identity, roles)) => {
            let user_id = create_user_id(identity, roles)?;
            Ok(HttpResponse::Ok().json(user_id))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn refresh_session(
    state: web::Data<State>,
    req: HttpRequest,
    identity_session: IdentitySession,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let fingerprint = state.iam().get_fingerprint(&req).await?;

    match state
        .iam()
        .refresh_session(user_id.user_id(), session_key.key(), &fingerprint)
        .await
    {
        Ok((identity, roles, session)) => {
            IdentityCookie::clear(&identity_session);
            create_user_id(identity, roles)?.to_session(&identity_session)?;
            SessionKey::from(session).to_session(&identity_session)?;
            Ok(HttpResponse::Ok().finish())
        }
        Err(e @ IAMError::SessionKeyConflict) => {
            // Preserve cookie and report a conflict error
            Err(e.into())
        }
        Err(e) => {
            IdentityCookie::clear(&identity_session);
            Err(e.into())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LogoutParams {
    force: bool,
}

pub async fn logout(
    state: web::Data<State>,
    logout_params: web::Json<LogoutParams>,
    identity_session: IdentitySession,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("logout {:?}, {:?}, {:?}", user_id, session_key, logout_params);

    state
        .iam()
        .invalidate_session(user_id.user_id(), session_key.key(), logout_params.force)
        .await?;
    IdentityCookie::clear(&identity_session);
    Ok(HttpResponse::Ok().finish())
}

#[derive(Serialize)]
struct RolesResponse {
    roles: Vec<String>,
}

pub async fn get_roles(state: web::Data<State>, identity_session: IdentitySession) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("get_roles {:?}, {:?}", user_id, session_key);

    //todo: check permission
    let roles = state.iam().get_roles().await?;
    Ok(HttpResponse::Ok().json(RolesResponse { roles }))
}

pub async fn create_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    testing_token: TestingToken,
    role: web::Path<String>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?;
    let user_id = UserId::from_session(&identity_session)?;
    log::info!("create_role[{:?},{:?},{:?}] {}", user_id, session_key, testing_token, role);

    state
        .iam()
        .check_permission_by_identity(user_id.as_ref().map(|u| u.user_id()), testing_token.token())
        .await?;

    state.iam().create_role(&role).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn delete_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    role: web::Path<String>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("delete_role {:?}, {:?}, {}", user_id, session_key, role);

    //todo: check permission
    state.iam().delete_role(&role).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn inherit_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    roles: web::Path<(String, String)>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("inherit_role {:?}, {:?}, {:?}", user_id, session_key, roles);

    //todo: check permission
    state.iam().inherit_role(&roles.0, &roles.1).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn disherit_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    roles: web::Path<(String, String)>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("disherit_role {:?}, {:?}, {:?}", user_id, session_key, roles);

    //todo: check permission
    state.iam().disherit_role(&roles.0, &roles.1).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn create_af_token(
    scope: web::Path<String>,
    af_session: AntiForgerySession,
    identity_session: IdentitySession,
) -> Result<HttpResponse, ActixError> {
    let user_id = UserId::from_session(&identity_session)?.map(|u| u.name().to_owned());
    log::info!("create_af_token: {:?}", scope);
    let af_issuer = AntiForgeryIssuer::new(&af_session, scope.to_owned(), user_id);

    #[derive(Serialize)]
    struct Response {
        token: String,
    };

    Ok(HttpResponse::Ok().json(Response {
        token: af_issuer.token().to_owned(),
    }))
}

pub async fn register_page(state: web::Data<State>, af_session: AntiForgerySession) -> Result<HttpResponse, ActixError> {
    let af_issuer = AntiForgeryIssuer::new(&af_session, "register_user".to_owned(), None);

    let mut ctx = tera::Context::new();
    ctx.insert("af", &af_issuer.token());
    let tera = state.tera();
    let body = tera
        .render("auth/register.html", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}
