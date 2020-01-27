use super::State;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};
use shine_core::authheader::BasicAuth;
use shine_core::session::{IdentityCookie, IdentitySession, SessionKey, UserId};
use shine_core::siteinfo::SiteInfo;

mod error;
pub mod identity;
pub mod session;

pub use self::error::*;

use identity::IdentityManager;
use session::SessionManager;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IAMConfig {
    pub password_pepper: String,
    pub storage_account: String,
    pub storage_account_key: String,
}

#[derive(Clone)]
pub struct IAM {
    identity: IdentityManager,
    session: SessionManager,
}

impl IAM {
    pub async fn new(config: IAMConfig) -> Result<Self, IAMError> {
        let identity = IdentityManager::new(&config).await?;
        let session = SessionManager::new(&config).await?;

        Ok(IAM { identity, session })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationParams {
    name: String,
    password: String,
    email: Option<String>,
}

pub async fn register_user(
    identity_session: IdentitySession,
    site: SiteInfo,
    registration_params: web::Json<RegistrationParams>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    log::info!("register {:?} {:?}", registration_params, site);
    IdentityCookie::clear(&identity_session);

    let RegistrationParams { name, password, email } = registration_params.into_inner();
    let identity = state.iam().identity.create_user(name, email, password).await?;
    let session = state.iam().session.create_session(&identity, site).await?;

    UserId::from(identity).to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn login_basic_auth(
    identity_session: IdentitySession,
    site: SiteInfo,
    auth: BasicAuth,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let (user_id, password) = (auth.user_id(), auth.password());
    log::info!("login {:?}, {:?}, {:?}", user_id, password, site);
    IdentityCookie::clear(&identity_session);

    let identity = state
        .iam()
        .identity
        .find_user_by_name_email(&user_id, password.as_deref())
        .await?;
    let session = state.iam().session.create_session(&identity, site).await?;

    UserId::from(identity).to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn refresh_session(
    identity_session: IdentitySession,
    site: SiteInfo,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("refresh session {:?}, {:?}, {:?}", user_id, session_key, site);

    unimplemented!()

    /*match state.identity_db().refresh_session(session_key.key(), &site).await {
        Ok((identity, session)) => {
            IdentityCookie::clear(&identity_session);
            UserId::from(identity).to_session(&identity_session)?;
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
    }*/
}

#[derive(Debug, Deserialize)]
pub struct LogoutParams {
    force: bool,
}

pub async fn logout(
    identity_session: IdentitySession,
    logout_params: web::Json<LogoutParams>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IAMError::SessionRequired)?;
    log::info!("logout {:?}, {:?}, {:?}", user_id, session_key, logout_params);

    unimplemented!()

    /*IdentityCookie::clear(&identity_session);
    if logout_params.force {
        state.identity_db().invalidate_all_sessions(session_key.key()).await?;
    } else {
        state.identity_db().invalidate_session(session_key.key()).await?;
    }

    Ok(HttpResponse::Ok().finish())*/
}
