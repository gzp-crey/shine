use crate::auth::iam::{
    identity::{Identity, UserIdentity},
    session::{Session, SessionData, SessionIndex, SessionIndexData},
    Fingerprint, IAMConfig, IAMError,
};
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use data_encoding;
use percent_encoding::utf8_percent_encode;
use rand::Rng;
use shine_core::{
    azure_utils,
    backoff::{self, Backoff, BackoffError},
};
use std::time::Duration;

const SESSION_KEY_LEN: usize = 32;
const KEY_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE64URL_NOPAD;

#[derive(Clone)]
pub struct SessionManager {
    db: TableStorage,
    time_to_live: ChronoDuration,
}

// Handling identites
impl SessionManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let db = TableStorage::new(table_service.clone(), "sessions");

        db.create_if_not_exists().await?;
        let time_to_live = ChronoDuration::hours(config.session_time_to_live_h as i64);

        Ok(SessionManager { db, time_to_live })
    }

    fn genrate_session_key(&self) -> String {
        let mut key_sequence = [0u8; SESSION_KEY_LEN];
        rand::thread_rng().fill(&mut key_sequence[..]);
        KEY_BASE_ENCODE.encode(&key_sequence)
    }

    fn get_minimum_refresh_date(&self) -> DateTime<Utc> {
        Utc::now() - self.time_to_live
    }

    async fn remove_index(&self, session: SessionIndex) {
        let session = session.into_entity();
        self.db
            .delete_entity(&session.partition_key, &session.row_key, session.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete session index {:?}: {}", session, e));
    }

    async fn try_insert_session(&self, identity: &UserIdentity, fingerprint: &Fingerprint) -> Result<Session, IAMError> {
        let id = &identity.core().id;
        let key = self.genrate_session_key();
        log::info!("Created new session key [{}] for {}", key, id);

        // fisrt insert index, it also ensures key uniqueness.
        let session_index = {
            let index = SessionIndex::new(&key, id);
            let index = match self.db.insert_entity(index.into_entity()).await {
                Ok(index) => index,
                Err(err) if azure_utils::is_precodition_error(&err) => return Err(IAMError::SessionKeyConflict),
                Err(err) => return Err(err.into()),
            };
            SessionIndex::from_entity(index)
        };

        let session = Session::new(id.to_owned(), key, fingerprint);
        let session = match self.db.insert_entity(session.into_entity()).await {
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

    async fn try_create_session(
        &self,
        identity: &UserIdentity,
        fingerprint: &Fingerprint,
    ) -> Result<Session, BackoffError<IAMError>> {
        let session = self
            .try_insert_session(identity, fingerprint)
            .await
            .map_err(IAMError::into_backoff)?;

        log::info!("New session: {:?}", session);
        return Ok(session);
    }

    /// Creates a new user session for the given identity.
    /// It is assumed that, the identity has been already authenticated.
    pub async fn create_session(&self, identity: &UserIdentity, fingerprint: &Fingerprint) -> Result<Session, IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_create_session(identity, fingerprint))
            .await
    }

    async fn find_session_by_id_key(&self, id: &str, key: &str) -> Result<Session, IAMError> {
        let (p, r) = Session::entity_keys(id, key);
        match self.db.get_entity::<SessionData>(&p, &r).await {
            Ok(Some(session)) => {
                if session.payload.disable_date().is_some() {
                    Err(IAMError::SessionExpired)
                } else {
                    Ok(Session::from_entity(session))
                }
            }
            Ok(None) => Err(IAMError::SessionExpired),
            Err(err) => Err(err.into()),
        }
    }

    async fn find_session_by_key(&self, key: &str) -> Result<(String, Session), IAMError> {
        let (p, r) = SessionIndex::entity_keys(key);
        let index = match self.db.get_entity::<SessionIndexData>(&p, &r).await {
            Ok(Some(index)) => SessionIndex::from_entity(index),
            Ok(None) => return Err(IAMError::SessionExpired),
            Err(err) => return Err(err.into()),
        };

        self.find_session_by_id_key(index.id(), index.key())
            .await
            .map(|session| (index.id().to_owned(), session))
    }

    async fn update_session(&self, session: Session) -> Result<Session, IAMError> {
        match self.db.update_entity(session.into_entity()).await {
            Ok(session) => Ok(Session::from_entity(session)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IAMError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn try_refresh_session_with_id_key(
        &self,
        id: &str,
        key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<Session, BackoffError<IAMError>> {
        let mut session = self.find_session_by_id_key(id, key).await.map_err(IAMError::into_backoff)?;

        // validate fingerprint
        if !session.check(fingerprint, self.get_minimum_refresh_date()) {
            session.disable();
            let _ = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Err(IAMError::SessionExpired.into_backoff())
        } else {
            session.refresh();
            let session = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Ok(session)
        }
    }

    /// Refresh the session when both the id and the key is known.
    /// In case of a compromised key the session is also disabled in the database.
    pub async fn refresh_session_with_id_key(&self, id: &str, key: &str, fingerprint: &Fingerprint) -> Result<Session, IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_refresh_session_with_id_key(id, key, fingerprint))
            .await
    }

    async fn try_validate_session_with_id_key(
        &self,
        id: &str,
        key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<Session, BackoffError<IAMError>> {
        let mut session = self.find_session_by_id_key(id, key).await.map_err(IAMError::into_backoff)?;

        // validate fingerprint
        if !session.check(fingerprint, self.get_minimum_refresh_date()) {
            session.disable();
            let _ = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Err(IAMError::SessionExpired.into_backoff())
        } else {
            Ok(session)
        }
    }

    /// Check if session key is valid when both the id and the key is known.
    /// In case of a compromised key the session is also disabled in the database.
    pub async fn validate_session_with_id_key(
        &self,
        id: &str,
        key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<Session, IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_validate_session_with_id_key(id, key, fingerprint))
            .await
    }

    async fn try_refresh_session_with_key(
        &self,
        key: &str,
        fingerprint: &Fingerprint,
    ) -> Result<(String, Session), BackoffError<IAMError>> {
        let (id, mut session) = self.find_session_by_key(key).await.map_err(IAMError::into_backoff)?;

        // validate fingerprint
        if session.check(fingerprint, self.get_minimum_refresh_date()) {
            session.disable();
            let _ = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Err(IAMError::SessionExpired.into_backoff())
        } else {
            session.refresh();
            let session = self.update_session(session).await.map_err(IAMError::into_backoff)?;
            Ok((id, session))
        }
    }

    /// Refresh the session when only the key is known.
    /// In case of a compromised key the session is also removed from the database.
    pub async fn refresh_session_with_key(&self, key: &str, fingerprint: &Fingerprint) -> Result<(String, Session), IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_refresh_session_with_key(key, fingerprint))
            .await
    }

    async fn try_invalidate_session(&self, id: &str, key: &str) -> Result<(), BackoffError<IAMError>> {
        let mut session = self.find_session_by_id_key(id, key).await.map_err(IAMError::into_backoff)?;
        //todo: securitiy consideration, this way a user might be forced to login
        // again knowing only the session key and hence the login handshake could be
        // triggered and captured.
        session.disable();
        self.update_session(session).await.map_err(IAMError::into_backoff).map(|_| ())
    }

    /// Invalidate a single session when both the id and the key is known.
    pub async fn invalidate_session(&self, id: &str, key: &str) -> Result<(), IAMError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_invalidate_session(id, key))
            .await
    }

    /// Invalidate all the sessions for an id
    pub async fn invalidate_all_session(&self, id: &str, active_key: Option<&str>) -> Result<(), IAMError> {
        // query all the active session
        let query = format!("PartitionKey eq 'id-{}' and Disabled eq ''", id);
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));
        let sessions = self.db.query_entities::<SessionData>(Some(&query)).await?;
        log::debug!("Sessions to invalidate: {:?}", sessions);

        let mut has_conflict = false;

        // perform the invalidation one-by-one with backoff to ensure a refresh won't keep the key alive.
        // due to conflicting updates.
        // The active key (if provided) is invalidated after all other sessions are invalidate to keep the
        // session alive on any error
        for session in sessions.into_iter() {
            let session = Session::from_entity(session);
            if let Some(key) = active_key {
                if key == session.key() {
                    // skip the active key
                    continue;
                }
            }

            match backoff::Exponential::new(3, Duration::from_micros(10))
                .async_execute(|_| self.try_invalidate_session(session.id(), session.key()))
                .await
            {
                Ok(_) => {}
                Err(IAMError::SessionKeyConflict) => {
                    has_conflict = true;
                }
                Err(err) => return Err(err),
            };
        }

        if has_conflict {
            Err(IAMError::SessionKeyConflict)
        } else if let Some(key) = active_key {
            self.invalidate_session(id, key).await
        } else {
            Ok(())
        }
    }
}
