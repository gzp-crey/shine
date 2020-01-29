use crate::auth::iam::{
    identity::{Identity, UserIdentity},
    session::{Session, SessionData, SessionIndex},
    IAMConfig, IAMError,
};
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use azure_sdk_storage_table::TableEntry;
use chrono::{DateTime, Utc};
use data_encoding;
use percent_encoding::{self, utf8_percent_encode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use shine_core::{
    azure_utils::{
        self,
        serde::{datetime, opt_datetime},
        table_storage::EmptyData,
    },
    backoff::{self, Backoff, BackoffError},
    session::SessionKey,
    siteinfo::SiteInfo,
};
use std::time::Duration;

const SESSION_KEY_LEN: usize = 32;
const KEY_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE64URL_NOPAD;

#[derive(Clone)]
pub struct SessionManager {
    db: TableStorage,
}

// Handling identites
impl SessionManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let session_db = TableStorage::new(table_service.clone(), "sessions");

        session_db.create_if_not_exists().await?;

        Ok(SessionManager { db: session_db })
    }

    fn genrate_session_key(&self) -> String {
        let mut key_sequence = [0u8; SESSION_KEY_LEN];
        rand::thread_rng().fill(&mut key_sequence[..]);
        KEY_BASE_ENCODE.encode(&key_sequence)
    }

    async fn remove_index(&self, session: SessionIndex) {
        let session = session.into_entity();
        self.db
            .delete_entry(&session.partition_key, &session.row_key, session.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete session index {:?}: {}", session, e));
    }

    async fn try_insert_session(&self, identity: &UserIdentity, site: &SiteInfo) -> Result<Session, IAMError> {
        let id = &identity.core().id;
        let key = self.genrate_session_key();
        log::info!("Created new session key [{}] for {}", key, id);

        // fisrt insert index, it also ensures key uniqueness.
        let session_index = {
            let index = SessionIndex::new(&key, id);
            let index = match self.db.insert_entry(index.into_entity()).await {
                Ok(index) => index,
                Err(err) if azure_utils::is_precodition_error(&err) => return Err(IAMError::SessionKeyConflict),
                Err(err) => return Err(err.into()),
            };
            SessionIndex::from_entity(index)
        };

        let session = Session::new(id.to_owned(), key, &site);
        let session = match self.db.insert_entry(session.into_entity()).await {
            Ok(session) => Session::from_entity(session),
            Err(err) => {
                self.remove_index(session_index).await;
                return Err(err.into());
            }
        };

        log::info!("New session for {}", id);
        log::debug!("Session index: {:?}", session_index);
        log::debug!("Session: {:?}", session);

        Ok(session)
    }

    async fn try_create_session(&self, identity: &UserIdentity, site: &SiteInfo) -> Result<Session, BackoffError<IAMError>> {
        let session = self
            .try_insert_session(identity, site)
            .await
            .map_err(IAMError::into_backoff)?;

        log::info!("New session: {:?}", session);
        return Ok(session);
    }

    /// Creates a new user session for the given identity.
    /// It is assumed that, the identity has been already authenticated.
    pub async fn create_session(&self, identity: &UserIdentity, site: &SiteInfo) -> Result<Session, IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_create_session(identity, site))
            .await
    }

    async fn find_session_by_id_key(&self, id: &str, key: &str) -> Result<Session, IAMError> {
        let (p, r) = Session::entity_keys(id, key);
        match self.db.get_entry::<SessionData>(&p, &r).await {
            Ok(Some(session)) => Ok(Session::from_entity(session)),
            Ok(None) => Err(IAMError::SessionExpired),
            Err(err) => Err(err.into()),
        }
    }

    async fn update_session(&self, session: Session) -> Result<Session, IAMError> {
        match self.db.update_entry(session.into_entity()).await {
            Ok(session) => Ok(Session::from_entity(session)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IAMError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn try_refresh_session_with_id_key(
        &self,
        id: &str,
        session_key: &str,
        site: &SiteInfo,
    ) -> Result<Session, BackoffError<IAMError>> {
        let mut session = self
            .find_session_by_id_key(id, session_key)
            .await
            .map_err(IAMError::into_backoff)?;

        // validate site
        if session.data().remote != site.remote() || session.data().agent != site.agent() {
            session.data_mut().disabled = Some(Utc::now());
            let _ = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Err(IAMError::SessionExpired.into_backoff())
        } else {
            session.data_mut().refresh_count += 1;
            session.data_mut().refreshed = Utc::now();
            let session = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Ok(session)
        }
    }

    /// Try to update the session and return a refreshed key.
    /// In case of a compromised session_key the session is also removed from the database.
    pub async fn refresh_session_with_id_key(&self, id: &str, session_key: &str, site: &SiteInfo) -> Result<Session, IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_refresh_session_with_id_key(id, session_key, site))
            .await
    }
}
/*
// Session handling
impl IdentityManager {

    /// Find a user and the session by the given session key.
    pub async fn find_user_by_session(&self, session_key: &str) -> Result<(UserIdentity, Session), IAMError> {
        let identity = {
            let (p, r) = SessionIndex::entity_keys(session_key);
            let query = format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r);
            let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));
            self.find_user_by_index(&query, None).await?
        };

        let session = {
            let partion_key = format!("{}", identity.id());
            let row_key = format!("session-{}", session_key);
            self.sessions
                .get_entry(&partion_key, &row_key)
                .await?
                .map(Session::from_entity)
                .ok_or(IAMError::IdentityNotFound)?
        };

        log::debug!("Session found {:?} for identity {:?}", session, identity);

        // session already disabled
        if session.data().disabled.is_some() {
            return Err(IAMError::SessionExpired);
        }

        Ok((identity, session))
    }

    async fn update_session(&self, session: Session) -> Result<Session, IAMError> {
        match self.sessions.update_entry(session.into_entity()).await {
            Ok(session) => Ok(Session::from_entity(session)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IAMError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn try_refresh_session(
        &self,
        session_key: &str,
        site: &SiteInfo,
    ) -> Result<(UserIdentity, Session), BackoffError<IAMError>> {
        let (identity, mut session) = self
            .find_user_by_session(session_key)
            .await
            .map_err(IAMError::into_backoff)?;

        // validate site
        if session.data().remote != site.remote() || session.data().agent != site.agent() {
            session.data_mut().disabled = Some(Utc::now());
            let _ = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Err(IAMError::SessionExpired.into_backoff())
        } else {
            session.data_mut().refresh_count += 1;
            session.data_mut().refreshed = Utc::now();
            let session = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Ok((identity, session))
        }
    }

    /// Try to update the session and return a refreshed key.
    /// In case of a compromised session_key the session is also removed from the database.
    pub async fn refresh_session(&self, session_key: &str, site: &SiteInfo) -> Result<(UserIdentity, Session), IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_refresh_session(session_key, site))
            .await
    }

    async fn try_invalidate_session(&self, session_key: &str) -> Result<(), BackoffError<IAMError>> {
        let (_, mut session) = self
            .find_user_by_session(session_key)
            .await
            .map_err(IAMError::into_backoff)?;

        session.data_mut().disabled = Some(Utc::now());
        self.update_session(session)
            .await
            .map_err(IAMError::into_backoff)
            .map(|_| ())
    }

    /// Invalidate the session by a key
    pub async fn invalidate_session(&self, session_key: &str) -> Result<(), IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_invalidate_session(session_key))
            .await
    }

    async fn invalidate_session_by_pr_key(&self, partition: &str, row: &str) -> Result<(), BackoffError<IAMError>> {
        if let Some(mut session) = self
            .sessions
            .get_entry::<SessionData>(partition, row)
            .await
            .map_err(|err| IAMError::from(err).into_backoff())?
        {
            if session.payload.disabled.is_none() {
                session.payload.disabled = Some(Utc::now())
            }

            // ignore any refresh, it is invalidated
            session.etag = None;
            match self.sessions.update_entry(session).await {
                Ok(_) => Ok(()),
                Err(err) if azure_utils::is_precodition_error(&err) => {
                    Err(BackoffError::Transient(IAMError::SessionKeyConflict))
                }
                Err(err) => Err(BackoffError::Permanent(err.into())),
            }
        } else {
            Ok(())
        }
    }

    /// Invalidate all the sessions corresponding to the same user as the key
    pub async fn invalidate_all_sessions(&self, session_key: &str) -> Result<(), IAMError> {
        let (identity, _) = self.find_user_by_session(session_key).await?;

        let query = format!(
            "PartitionKey eq '{}' and RowKey gt 'session-' and RowKey lt 'session_' and Disabled eq ''",
            identity.id()
        );
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));
        let sessions = self.sessions.query_entries::<EmptyData>(Some(&query)).await?;
        log::debug!("sessions: {:?}", sessions);
        for session in sessions.into_iter() {
            if let Err(err) = backoff::Exponential::new(3, Duration::from_micros(10))
                .async_execute(|_| self.invalidate_session_by_pr_key(&session.partition_key, &session.row_key))
                .await
            {
                log::warn!(
                    "Failed to invalidate session: {},{}: {:?}",
                    session.partition_key,
                    session.row_key,
                    err
                )
            }
        }

        Ok(())
    }
}
*/
