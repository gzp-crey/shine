use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use shine_core::iplocation::{IpCachedLocation, IpCachedLocationConfig, IpLocationIpDataCo, IpLocationIpDataCoConfig};
use std::time::Duration;

mod error;
pub mod fingerprint;
pub mod identity;
pub mod role;
pub mod session;

pub use self::error::*;

use fingerprint::Fingerprint;
use identity::{Identity, IdentityManager, UserIdentity};
use role::{RoleManager, Roles};
use session::{Session, SessionManager};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IAMConfig {
    pub password_pepper: String,
    pub storage_account: String,
    pub storage_account_key: String,
    pub ipdataco_key: String,
    pub session_time_to_live_h: u16,
}

#[derive(Clone)]
pub struct IAM {
    identity: IdentityManager,
    session: SessionManager,
    role: RoleManager,
    iplocation: IpCachedLocation,
}

impl IAM {
    pub async fn new(config: IAMConfig) -> Result<Self, IAMError> {
        let identity = IdentityManager::new(&config).await?;
        let session = SessionManager::new(&config).await?;
        let role = RoleManager::new(&config).await?;

        let cfg = IpLocationIpDataCoConfig {
            api_key: config.ipdataco_key.clone(),
        };
        let provider = IpLocationIpDataCo::new(cfg);
        let cfg = IpCachedLocationConfig {
            storage_account: config.storage_account.clone(),
            storage_account_key: config.storage_account_key.clone(),
            table_name: "ipcache".to_owned(),
            time_to_live: Duration::from_secs(12 * 60 * 60),
        };
        let iplocation = IpCachedLocation::new(provider, cfg).await?;

        Ok(IAM {
            identity,
            session,
            role,
            iplocation,
        })
    }

    pub async fn get_fingerprint(&self, req: &HttpRequest) -> Result<Fingerprint, IAMError> {
        Fingerprint::new(req, &self.iplocation).await
    }

    pub async fn register_user(
        &self,
        name: &str,
        email: Option<&str>,
        password: &str,
        fingerprint: &Fingerprint,
    ) -> Result<(UserIdentity, Roles, Session), IAMError> {
        let identity = self.identity.create_user(name, email, password).await?;
        let session = self.session.create_session(&identity, fingerprint).await?;
        let roles = self.role.get_roles_by_identity(&identity.core().id, true).await?;

        Ok((identity, roles, session))
    }

    pub async fn login_name_email(
        &self,
        name_email: &str,
        password: &str,
        fingerprint: &Fingerprint,
    ) -> Result<(UserIdentity, Roles, Session), IAMError> {
        let identity = self.identity.find_user_by_name_email(name_email, Some(&password)).await?;
        let session = self.session.create_session(&identity, fingerprint).await?;
        let roles = self.role.get_roles_by_identity(&identity.core().id, true).await?;

        Ok((identity, roles, session))
    }

    pub async fn validate_session(
        &self,
        user_id: &str,
        session_key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<(UserIdentity, Roles), IAMError> {
        let _session = self
            .session
            .validate_session_with_id_key(user_id, session_key, fingerprint)
            .await?;
        let identity = self.identity.find_user_by_id(user_id).await?;
        let roles = self.role.get_roles_by_identity(&identity.core().id, true).await?;

        Ok((identity, roles))
    }

    pub async fn refresh_session(
        &self,
        user_id: &str,
        session_key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<(UserIdentity, Roles, Session), IAMError> {
        let session = self
            .session
            .refresh_session_with_id_key(user_id, session_key, fingerprint)
            .await?;
        let identity = self.identity.find_user_by_id(user_id).await?;
        let roles = self.role.get_roles_by_identity(&identity.core().id, true).await?;

        Ok((identity, roles, session))
    }

    pub async fn refresh_session_by_key(
        &self,
        session_key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<(UserIdentity, Roles, Session), IAMError> {
        let (user_id, session) = self.session.refresh_session_with_key(session_key, fingerprint).await?;
        let identity = self.identity.find_user_by_id(&user_id).await?;
        let roles = self.role.get_roles_by_identity(&identity.core().id, true).await?;

        Ok((identity, roles, session))
    }

    pub async fn invalidate_session(&self, user_id: &str, session_key: &str, invalidate_all: bool) -> Result<(), IAMError> {
        if invalidate_all {
            self.session.invalidate_all_session(user_id, Some(session_key)).await
        } else {
            self.session.invalidate_session(user_id, session_key).await
        }
    }
}
