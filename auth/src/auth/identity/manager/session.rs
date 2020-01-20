use super::*;
use data_encoding;
use percent_encoding::{self, utf8_percent_encode};
use rand::Rng;

const SESSION_KEY_LEN: usize = 32;
const KEY_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE64URL_NOPAD;

// Session handling
impl IdentityManager {
    fn genrate_session_key(&self) -> String {
        let mut key_sequence = [0u8; SESSION_KEY_LEN];
        rand::thread_rng().fill(&mut key_sequence[..]);
        KEY_BASE_ENCODE.encode(&key_sequence)
    }

    async fn try_insert_session(&self, identity: &IdentityEntry, site: &SiteInfo) -> Result<SessionEntry, IdentityError> {
        let user_id = identity.user_id();
        let key = self.genrate_session_key();
        log::info!("Created new session key [{}] for {}", key, user_id);

        let session = SessionEntry::new(user_id.to_owned(), key, &site);
        match self.sessions.insert_entry(session.into_entry()).await {
            Ok(session) => Ok(SessionEntry::from_entry(session)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IdentityError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    async fn delete_session(&self, session: TableEntry<Session>) {
        self.users
            .delete_entry(&session.partition_key, &session.row_key, session.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete session {:?}: {}", session, e));
    }

    async fn try_insert_session_index(&self, session: &SessionEntry) -> Result<SessionIndexEntry, IdentityError> {
        let session_index = SessionIndexEntry::from_identity(session);
        match self.indices.insert_entry(session_index.into_entry()).await {
            Ok(session_index) => Ok(SessionIndexEntry::from_entry(session_index)),
            Err(err) if azure_utils::is_precodition_error(&err) => Err(IdentityError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn try_create_session(
        &self,
        identity: &IdentityEntry,
        site: &SiteInfo,
    ) -> Result<SessionEntry, BackoffError<IdentityError>> {
        let session = match self.try_insert_session(identity, site).await {
            Ok(session) => session,
            Err(IdentityError::SessionKeyConflict) => return Err(BackoffError::Transient(IdentityError::SessionKeyConflict)),
            Err(err) => return Err(BackoffError::Permanent(err)),
        };

        let session_index = match self.try_insert_session_index(&session).await {
            Ok(index) => index,
            Err(IdentityError::SessionKeyConflict) => {
                self.delete_session(session.into_entry()).await;
                return Err(BackoffError::Transient(IdentityError::SessionKeyConflict));
            }
            Err(err) => {
                self.delete_session(session.into_entry()).await;
                return Err(BackoffError::Permanent(err));
            }
        };

        log::info!("New session: {:?}", session);
        log::debug!("Session index: {:?}", session_index);
        return Ok(session);
    }

    pub async fn create_session(&self, identity: &IdentityEntry, site: SiteInfo) -> Result<SessionEntry, IdentityError> {
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_create_session(identity, &site))
            .await
    }

    pub async fn find_identity_by_session(&self, session_key: &str) -> Result<(IdentityEntry, SessionEntry), IdentityError> {
        let identity = {
            let query = format!(
                "PartitionKey eq '{}' and RowKey eq '{}'",
                NameIndexEntry::generate_partion_key(session_key),
                session_key
            );
            let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));
            self.find_identity_by_index(&query, None).await?
        };

        let session = {
            let partion_key = format!("{}", identity.user_id());
            let row_key = format!("session-{}", session_key);
            self.sessions
                .get_entry(&partion_key, &row_key)
                .await?
                .map(SessionEntry::from_entry)
                .ok_or(IdentityError::UserNotFound)?
        };

        log::debug!("Session found {:?} for identity {:?}", session, identity);
        Ok((identity, session))
    }

    pub async fn update_session(&self, _session: SessionEntry) -> Result<(), IdentityError> {
        unimplemented!()
    }
}
