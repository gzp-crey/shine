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

use identity::{IdentityManager, UserIdentity};
use session::{Session, SessionManager};

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

    pub async fn register_user(
        &self,
        name: &str,
        email: Option<&str>,
        password: &str,
        site: &SiteInfo,
    ) -> Result<(UserIdentity, Session), IAMError> {
        let identity = self.identity.create_user(name, email, password).await?;
        let session = self.session.create_session(&identity, site).await?;

        Ok((identity, session))
    }

    pub async fn login_name_email(
        &self,
        name_email: &str,
        password: &str,
        site: &SiteInfo,
    ) -> Result<(UserIdentity, Session), IAMError> {
        let identity = self.identity.find_user_by_name_email(name_email, Some(&password)).await?;
        let session = self.session.create_session(&identity, site).await?;

        Ok((identity, session))
    }

    pub async fn refresh_session_by_id_key(
        &self,
        user_id: &str,
        session_key: &str,
        site: &SiteInfo,
    ) -> Result<(UserIdentity, Session), IAMError> {
        let session = self.session.refresh_session_with_id_key(user_id, session_key, site).await?;
        let identity = self.identity.find_user_by_id(user_id).await?;
        Ok((identity, session))
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
    let RegistrationParams { name, password, email } = registration_params.into_inner();

    IdentityCookie::clear(&identity_session);

    let (identity, session) = state.iam().register_user(&name, email.as_deref(), &password, &site).await?;

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
    log::info!("login {:?}, {:?}", auth, site);

    let user_id = auth.user_id();
    let password = auth.password().ok_or(IAMError::PasswordNotMatching)?;

    IdentityCookie::clear(&identity_session);

    let (identity, session) = state.iam().login_name_email(&user_id, password, &site).await?;

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

    match state
        .iam()
        .refresh_session_by_id_key(user_id.user_id(), session_key.key(), &site)
        .await
    {
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
    }
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
