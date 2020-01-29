use serde::{Deserialize, Serialize};
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

    pub async fn refresh_session_by_key(&self, session_key: &str, site: &SiteInfo) -> Result<(UserIdentity, Session), IAMError> {
        let (user_id, session) = self.session.refresh_session_with_key(session_key, site).await?;
        let identity = self.identity.find_user_by_id(&user_id).await?;
        Ok((identity, session))
    }

    pub async fn invalidate_session(&self, session_key: &str, invalidate_all: bool) -> Result<(), IAMError> {
        unimplemented!()
    }
}
