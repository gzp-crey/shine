use super::super::{error::IdentityError, IdentityManager};
use super::{Identity, IdentityIndex, IdentityIndexData, IdentityIndexedId, UserIdentity};
use azure_sdk_storage_table::TableEntry;
use chrono::{DateTime, Utc};
use data_encoding;
use percent_encoding::{self, utf8_percent_encode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use shine_core::{
    azure_utils::{self, table_storage::EmptyData},
    backoff::{self, Backoff, BackoffError},
    session::SessionKey,
    siteinfo::SiteInfo,
};
use std::time::Duration;

const SESSION_KEY_LEN: usize = 32;
const KEY_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE64URL_NOPAD;

/// Data associated to a session
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionData {
    pub remote: String,
    pub agent: String,

    pub issued: DateTime<Utc>,
    pub refresh_count: u64,
    pub refreshed: DateTime<Utc>,
    pub disabled: Option<DateTime<Utc>>,
}

/// The session of a user. Only users may have a session, other type of identites cannot log in and thus cannot
/// have session.
#[derive(Debug)]
pub struct Session(TableEntry<SessionData>);

impl Session {
    pub fn entity_keys(user_id: &str, key: &str) -> (String, String) {
        (user_id.to_owned(), format!("session-{}", key))
    }

    pub fn new(user_id: String, key: String, site: &SiteInfo) -> Session {
        let (partition_key, row_key) = Self::entity_keys(&user_id, &key);

        Session(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: SessionData {
                remote: site.remote().to_string(),
                agent: site.agent().to_string(),
                issued: Utc::now(),
                refresh_count: 0,
                refreshed: Utc::now(),
                disabled: None,
            },
        })
    }

    pub fn from_entity(entity: TableEntry<SessionData>) -> Self {
        Self(entity)
    }

    pub fn into_entity(self) -> TableEntry<SessionData> {
        self.0
    }

    pub fn data(&self) -> &SessionData {
        &self.0.payload
    }

    pub fn data_mut(&mut self) -> &mut SessionData {
        &mut self.0.payload
    }

    pub fn id(&self) -> &str {
        &self.0.partition_key
    }

    pub fn key(&self) -> &str {
        &self.0.row_key.splitn(2, '-').skip(1).next().unwrap()
    }
}

impl From<Session> for SessionKey {
    fn from(session: Session) -> SessionKey {
        SessionKey::new(session.key().to_string())
    }
}

/// Data associated to an identity index by session
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SessionIndexData {
    #[serde(flatten)]
    pub indexed_id: IdentityIndexedId,
}

impl IdentityIndexData for SessionIndexData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

/// Index (user)identity by session
#[derive(Debug)]
struct SessionIndex(TableEntry<SessionIndexData>);

impl SessionIndex {
    pub fn from_identity(session: &Session) -> Self {
        let key = session.key();
        let (partition_key, row_key) = Self::entity_keys(&key);

        Self(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: SessionIndexData {
                indexed_id: IdentityIndexedId {
                    identity_id: session.id().to_owned(),
                },
            },
        })
    }
}

impl IdentityIndex for SessionIndex {
    type Index = SessionIndexData;

    fn entity_keys(key: &str) -> (String, String) {
        (format!("session-{}", &key[0..2]), key.to_string())
    }

    fn from_entity(entity: TableEntry<SessionIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntry<SessionIndexData> {
        self.0
    }

    fn into_data(self) -> SessionIndexData {
        self.0.payload
    }

    fn data(&self) -> &SessionIndexData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut SessionIndexData {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.row_key
    }
}

// Session handling
impl IdentityManager {
    fn genrate_session_key(&self) -> String {
        let mut key_sequence = [0u8; SESSION_KEY_LEN];
        rand::thread_rng().fill(&mut key_sequence[..]);
        KEY_BASE_ENCODE.encode(&key_sequence)
    }

    async fn try_insert_session(&self, identity: &UserIdentity, site: &SiteInfo) -> Result<Session, IdentityError> {
        let id = identity.id();
        let key = self.genrate_session_key();
        log::info!("Created new session key [{}] for {}", key, id);

        let session = Session::new(id.to_owned(), key, &site);
        match self.sessions.insert_entry(session.into_entity()).await {
            Ok(session) => Ok(Session::from_entity(session)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IdentityError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn remove_session(&self, session: Session) {
        let session = session.into_entity();
        self.sessions
            .delete_entry(&session.partition_key, &session.row_key, session.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete session {:?}: {}", session, e));
    }

    async fn try_insert_session_index(&self, session: &Session) -> Result<SessionIndex, IdentityError> {
        let session_index = SessionIndex::from_identity(session);
        match self.indices.insert_entry(session_index.into_entity()).await {
            Ok(session_index) => Ok(SessionIndex::from_entity(session_index)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IdentityError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn try_create_session(&self, identity: &UserIdentity, site: &SiteInfo) -> Result<Session, BackoffError<IdentityError>> {
        let session = self
            .try_insert_session(identity, site)
            .await
            .map_err(IdentityError::into_backoff)?;

        let session_index = match self.try_insert_session_index(&session).await {
            Ok(index) => index,
            Err(e) => {
                self.remove_session(session).await;
                return Err(e.into_backoff());
            }
        };

        log::info!("New session: {:?}", session);
        log::debug!("Session index: {:?}", session_index);
        return Ok(session);
    }

    /// Creates a new user session for the given identity.
    /// It is assumed that, the identity has been already authenticated.
    pub async fn create_session(&self, identity: &UserIdentity, site: SiteInfo) -> Result<Session, IdentityError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_create_session(identity, &site))
            .await
    }

    /// Find a user and the session by the given session key.
    pub async fn find_user_by_session(&self, session_key: &str) -> Result<(UserIdentity, Session), IdentityError> {
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
                .ok_or(IdentityError::IdentityNotFound)?
        };

        log::debug!("Session found {:?} for identity {:?}", session, identity);
        Ok((identity, session))
    }

    async fn update_session(&self, session: Session) -> Result<Session, IdentityError> {
        match self.sessions.update_entry(session.into_entity()).await {
            Ok(session) => Ok(Session::from_entity(session)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IdentityError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn try_refresh_session(
        &self,
        session_key: &str,
        site: &SiteInfo,
    ) -> Result<(UserIdentity, Session), BackoffError<IdentityError>> {
        let (identity, mut session) = self
            .find_user_by_session(session_key)
            .await
            .map_err(IdentityError::into_backoff)?;

        // session already disabled
        if session.data().disabled.is_some() {
            return Err(IdentityError::SessionExpired.into_backoff());
        }

        // validate site
        if session.data().remote != site.remote() || session.data().agent != site.agent() {
            session.data_mut().disabled = Some(Utc::now());
            let _ = self.update_session(session).await.map_err(IdentityError::into_backoff)?;
            Err(IdentityError::SessionExpired.into_backoff())
        } else {
            session.data_mut().refresh_count += 1;
            session.data_mut().refreshed = Utc::now();
            let session = self.update_session(session).await.map_err(IdentityError::into_backoff)?;
            Ok((identity, session))
        }
    }

    /// Try to update the session and return a refreshed key.
    /// In case of a compromised session_key the session is also removed from the database.
    pub async fn refresh_session(&self, session_key: &str, site: &SiteInfo) -> Result<(UserIdentity, Session), IdentityError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_refresh_session(session_key, site))
            .await
    }

    async fn try_invalidate_session(&self, session_key: &str) -> Result<(), BackoffError<IdentityError>> {
        let (_, mut session) = self
            .find_user_by_session(session_key)
            .await
            .map_err(IdentityError::into_backoff)?;

        session.data_mut().disabled = Some(Utc::now());
        self.update_session(session)
            .await
            .map_err(IdentityError::into_backoff)
            .map(|_| ())
    }

    /// Invalidate the session by a key
    pub async fn invalidate_session(&self, session_key: &str) -> Result<(), IdentityError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_invalidate_session(session_key))
            .await
    }

    async fn invalidate_session_by_pr_key(&self, partition: &str, row: &str) -> Result<(), BackoffError<IdentityError>> {
        log::debug!("invalidate session: {},{}", partition, row);
        if let Some(mut session) = self
            .sessions
            .get_entry::<SessionData>(partition, row)
            .await
            .map_err(|err| IdentityError::from(err).into_backoff())?
        {
            log::debug!("invalidate session: {:?}", session);
            if session.payload.disabled.is_none() {
                session.payload.disabled = Some(Utc::now())
            }

            match self.sessions.update_entry(session).await {
                Ok(_) => Ok(()),
                Err(err) if azure_utils::is_precodition_error(&err) => {
                    Err(BackoffError::Transient(IdentityError::SessionKeyConflict))
                }
                Err(err) => Err(BackoffError::Permanent(err.into())),
            }
        } else {
            Ok(())
        }
    }

    /// Invalidate all the sessions corresponding to the same user as the key
    pub async fn invalidate_all_sessions(&self, session_key: &str) -> Result<(), IdentityError> {
        let (identity, _) = self.find_user_by_session(session_key).await?;

        let query = format!(
            "PartitionKey eq '{}' and RowKey gt 'session-' and RowKey lt 'session_'",
            identity.id()
        );
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));
        let sessions = self.sessions.query_entries::<EmptyData>(Some(&query)).await?;
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
