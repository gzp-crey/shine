use super::iam::IAMError;
use super::utils::create_user_id;
use super::State;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use shine_core::kernel::anti_forgery::{
    AntiForgeryIdentity, AntiForgeryIssuer, AntiForgerySession, AntiForgeryValidator,
};
use shine_core::kernel::identity::{IdentityCookie, IdentitySession, SessionKey, UserId};
use shine_core::kernel::response::APIResult;
use shine_core::requestinfo::{BasicAuth, RemoteInfo, TestingToken};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationParams {
    name: String,
    password: String,
    email: Option<String>,
    af: String,
}

pub async fn register_user(
    state: web::Data<State>,
    identity_session: IdentitySession,
    af_session: AntiForgerySession,
    remote_info: RemoteInfo,
    testing_token: TestingToken,
    params: web::Json<RegistrationParams>,
) -> APIResult {
    let params = params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;
    log::info!(
        "register_user[{:?},{:?}]: {:?}",
        testing_token.token(),
        fingerprint,
        params
    );

    if testing_token.is_valid() {
        state
            .iam()
            .check_permission_by_testing_token(testing_token.token())
            .await?;
    } else {
        let _ = AntiForgeryValidator::validate(&af_session, &params.af, AntiForgeryIdentity::Ignore)
            .map_err(IAMError::from)?;
    }

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = state
        .iam()
        .register_user(&params.name, params.email.as_deref(), &params.password, &fingerprint)
        .await?;

    create_user_id(identity, roles)?.to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn login_basic_auth(
    state: web::Data<State>,
    identity_session: IdentitySession,
    remote_info: RemoteInfo,
    auth: BasicAuth,
) -> APIResult {
    let user_id = auth.user_id();
    let password = auth.password().ok_or(IAMError::PasswordNotMatching)?;
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;

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
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
    key_params: web::Json<RefreshKeyParams>,
) -> APIResult {
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;

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
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;

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
    remote_info: RemoteInfo,
    identity_session: IdentitySession,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let fingerprint = state.iam().get_fingerprint(&remote_info).await?;

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
) -> APIResult {
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

pub async fn get_roles(state: web::Data<State>, identity_session: IdentitySession) -> APIResult {
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
    query: web::Path<String>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?;
    let user_id = UserId::from_session(&identity_session)?;
    log::info!(
        "create_role[{:?},{:?},{:?}] {}",
        user_id,
        session_key,
        testing_token,
        query
    );

    state
        .iam()
        .check_permission_by_identity(user_id.as_ref().map(|u| u.user_id()), testing_token.token())
        .await?;

    state.iam().create_role(&query).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn delete_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    query: web::Path<String>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("delete_role {:?}, {:?}, {}", user_id, session_key, query);

    //todo: check permission
    state.iam().delete_role(&query).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn inherit_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    query: web::Path<(String, String)>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("inherit_role {:?}, {:?}, {:?}", user_id, session_key, query);

    //todo: check permission
    state.iam().inherit_role(&query.0, &query.1).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn disherit_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    query: web::Path<(String, String)>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("disherit_role {:?}, {:?}, {:?}", user_id, session_key, query);

    //todo: check permission
    state.iam().disherit_role(&query.0, &query.1).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_user_roles(
    state: web::Data<State>,
    identity_session: IdentitySession,
    query: web::Path<String>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("get_user_roles {:?}, {:?}, {:?}", user_id, session_key, query);

    //todo: check permission
    let roles = state.iam().get_identity_roles(&query, true).await?;
    Ok(HttpResponse::Ok().json(roles))
}

pub async fn add_user_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    query: web::Path<(String, String)>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("add_user_role {:?}, {:?}, {:?}", user_id, session_key, query);

    //todo: check permission
    let roles = state.iam().add_identity_role(&query.0, &query.1).await?;
    Ok(HttpResponse::Ok().json(roles))
}

pub async fn remove_user_role(
    state: web::Data<State>,
    identity_session: IdentitySession,
    query: web::Path<(String, String)>,
) -> APIResult {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("remove_user_role {:?}, {:?}, {:?}", user_id, session_key, query);

    //todo: check permission
    let roles = state.iam().remove_identity_role(&query.0, &query.1).await?;
    Ok(HttpResponse::Ok().json(roles))
}

pub async fn create_af_token(af_session: AntiForgerySession, identity_session: IdentitySession) -> APIResult {
    let user_id = UserId::from_session(&identity_session)?.map(|u| u.name().to_owned());
    log::info!("create_af_token");

    #[derive(Serialize)]
    struct Response {
        token: String,
    };

    let token = AntiForgeryIssuer::issue(&af_session, user_id);
    Ok(HttpResponse::Ok().json(Response { token }))
}
