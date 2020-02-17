use super::iam::{
    identity::{Identity, UserIdentity},
    role::Roles,
    IAMError,
};
use super::State;
use actix_web::HttpRequest;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};
use shine_core::requestinfo::BasicAuth;
use shine_core::session::{IdentityCookie, IdentitySession, SessionKey, UserId};

fn create_user_id(user: UserIdentity, roles: Roles) -> UserId {
    let data = user.into_data();
    UserId::new(data.core.id, data.core.name, roles)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationParams {
    name: String,
    password: String,
    email: Option<String>,
}

pub async fn register_user(
    req: HttpRequest,
    identity_session: IdentitySession,
    registration_params: web::Json<RegistrationParams>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let RegistrationParams { name, password, email } = registration_params.into_inner();
    let fingerprint = state.iam().get_fingerprint(&req).await?;
    log::info!("register_user: {}, {}, {:?}, {:?}", name, password, email, fingerprint);

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = state
        .iam()
        .register_user(&name, email.as_deref(), &password, &fingerprint)
        .await?;

    create_user_id(identity, roles).to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn login_basic_auth(
    req: HttpRequest,
    identity_session: IdentitySession,
    auth: BasicAuth,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let user_id = auth.user_id();
    let password = auth.password().ok_or(IAMError::PasswordNotMatching)?;
    let fingerprint = state.iam().get_fingerprint(&req).await?;

    IdentityCookie::clear(&identity_session);

    let (identity, roles, session) = state.iam().login_name_email(&user_id, password, &fingerprint).await?;

    create_user_id(identity, roles).to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Deserialize)]
pub struct RefreshKeyParams {
    key: String,
}

pub async fn refresh_session_by_key(
    req: HttpRequest,
    identity_session: IdentitySession,
    key_params: web::Json<RefreshKeyParams>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let fingerprint = state.iam().get_fingerprint(&req).await?;

    match state.iam().refresh_session_by_key(&key_params.key, &fingerprint).await {
        Ok((identity, roles, session)) => {
            IdentityCookie::clear(&identity_session);
            create_user_id(identity, roles).to_session(&identity_session)?;
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
    req: HttpRequest,
    identity_session: IdentitySession,
    state: web::Data<State>,
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
            let user_id = create_user_id(identity, roles);
            Ok(HttpResponse::Ok().json(user_id))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn refresh_session(
    req: HttpRequest,
    identity_session: IdentitySession,
    state: web::Data<State>,
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
            create_user_id(identity, roles).to_session(&identity_session)?;
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
    logout_params: web::Json<LogoutParams>,
    identity_session: IdentitySession,
    state: web::Data<State>,
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

pub async fn get_roles(identity_session: IdentitySession, state: web::Data<State>) -> Result<HttpResponse, ActixError> {
    use gremlin_client::{GremlinClient, Vertex};
    let client = GremlinClient::connect("localhost").map_err(IAMError::from)?;

    let results = client
        .execute("g.V(param)", &[("param", &1)]).map_err(IAMError::from)?
        .filter_map(Result::ok)
        .map(|f| f.take::<Vertex>())
        .collect::<Result<Vec<Vertex>, _>>().map_err(IAMError::from)?;

    Ok(HttpResponse::Ok().body(format!("{:?}", results)))
    /*let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("get_roles {:?}, {:?}", user_id, session_key);

    //todo: check permission

    let roles = state.iam().get_roles().await?;

    Ok(HttpResponse::Ok().json(roles))*/
}
