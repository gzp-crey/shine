use super::{
    error::IdentityError,
    identityentry::{EmailIndexEntry, EmptyEntry, Identity, IdentityEntry, IdentityIndex, NameIndexEntry},
    sessionentry::{Session, SessionEntry, SessionIndexEntry},
    IdentityConfig,
};
use argon2;
use azure_sdk_core::errors::AzureError;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use azure_utils::idgenerator::{IdSequence, SyncCounterConfig, SyncCounterStore};
use block_modes::{
    block_padding::Pkcs7,
    {BlockMode, Cbc},
};
use blowfish::Blowfish;
use data_encoding::BASE64;
use percent_encoding::{self, utf8_percent_encode};
use rand::{distributions::Alphanumeric, Rng};
use shine_core::siteinfo::SiteInfo;
use std::{iter, str};
use validator::validate_email;

const SALT_LEN: usize = 32;
const CIPHER_IV_LEN: usize = 8;
const SESSION_KEY_LEN: usize = 32;
type Cipher = Cbc<Blowfish, Pkcs7>;

const ID_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE32_NOPAD;
const KEY_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE64URL_NOPAD;

#[derive(Clone)]
pub struct IdentityManager {
    password_pepper: String,

    user_id_secret: Vec<u8>,
    user_id_generator: IdSequence,

    users: TableStorage,
    indices: TableStorage,
    sessions: TableStorage,
}

impl IdentityManager {
    async fn delete_index<K>(&self, index: TableEntry<K>) {
        self.indices
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }

    async fn find_identity_by_index(&self, query: &str, password: Option<&str>) -> Result<IdentityEntry, IdentityError> {
        let index = self.indices.query_entries::<IdentityIndex>(Some(&query)).await?;
        assert!(index.len() <= 1);
        let index = index.first().ok_or(IdentityError::UserNotFound)?;

        let user_id = &index.payload.user_id;
        let partion_key = IdentityEntry::generate_partion_key(&user_id);
        let identity = self.users.get_entry(&partion_key, &user_id).await?;
        let identity = identity.map(IdentityEntry::from_entry).ok_or(IdentityError::UserNotFound)?;

        if let Some(password) = password {
            // check password if provided, this is a low level function and it's ok if no password was
            if !argon2::verify_encoded(&identity.data().password_hash, password.as_bytes())? {
                return Err(IdentityError::PasswordNotMatching);
            }
        }

        Ok(identity)
    }
}

// Handling identites
impl IdentityManager {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let indices = TableStorage::new(table_service.clone(), "userIndices");
        let sessions = TableStorage::new(table_service.clone(), "userSessions");

        indices.create_if_not_exists().await?;
        users.create_if_not_exists().await?;
        sessions.create_if_not_exists().await?;

        let user_id_generator = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            IdSequence::new(id_counter.clone(), "userid").with_granularity(10)
        };
        let user_id_secret = BASE64.decode(config.user_id_secret.as_bytes())?;

        Ok(IdentityManager {
            password_pepper: config.password_pepper.clone(),
            user_id_secret,
            users,
            indices,
            sessions,
            user_id_generator,
        })
    }

    fn user_id_cipher(&self, salt: &str) -> Result<Cipher, IdentityError> {
        let cipher = Cipher::new_var(&self.user_id_secret, &salt.as_bytes()[0..CIPHER_IV_LEN])?;
        Ok(cipher)
    }

    async fn generate_user_id(&self, sequence_id: u64) -> Result<(String, String, String), IdentityError> {
        let sequence_id = format!("{:0>10}", sequence_id);
        let mut rng = rand::thread_rng();
        let salt = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(SALT_LEN)
            .collect::<String>();

        let cipher = self.user_id_cipher(&salt)?;
        let id = cipher.encrypt_vec(sequence_id.as_bytes());
        let id = ID_BASE_ENCODE.encode(&id);
        debug_assert_eq!(self.decode_user_id(&id, &salt).ok(), Some(sequence_id.clone()));

        Ok((id, sequence_id, salt))
    }

    fn decode_user_id(&self, id: &str, salt: &str) -> Result<String, IdentityError> {
        let id = ID_BASE_ENCODE.decode(id.as_bytes())?;
        let cipher = self.user_id_cipher(salt)?;
        let id = cipher.decrypt_vec(&id)?;
        let sequence_id = str::from_utf8(&id)?.to_string();
        log::info!("Decoded user id:[{}]", sequence_id);
        Ok(sequence_id)
    }

    async fn try_insert_user(
        &self,
        sequence_id: u64,
        name: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<IdentityEntry, IdentityError> {
        let (id, sequence_id, salt) = self.generate_user_id(sequence_id).await?;
        let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &argon2::Config::default())?;
        log::info!("Created new user id:{}, pwh:{}", id, password_hash);
        let identity = IdentityEntry::new(
            id,
            sequence_id,
            salt,
            name.to_owned(),
            email.map(|e| e.to_owned()),
            password_hash,
        );

        match self.users.insert_entry(identity.into_entry()).await {
            Ok(identity) => Ok(IdentityEntry::from_entry(identity)),
            Err(err) if is_conflict(&err) => Err(IdentityError::UserIdConflict),
            Err(err) => Err(IdentityError::from(err)),
        }
    }

    async fn insert_user(&self, name: &str, password: &str, email: Option<&str>) -> Result<IdentityEntry, IdentityError> {
        let sequence_id = self.user_id_generator.get().await?;
        let mut retry = 3usize;
        //todo: use backoff::retry
        loop {
            let identity = match self.try_insert_user(sequence_id, name, password, email).await {
                Ok(identity) => identity,
                Err(IdentityError::UserIdConflict) if retry > 0 => {
                    retry -= 1;
                    log::info!("Retrying ({}) user creation with sequence_id: {}", retry, sequence_id);
                    continue;
                }
                Err(err) => return Err(err),
            };

            log::info!("New user: {:?}", identity);
            return Ok(identity);
        }
    }

    async fn delete_user(&self, identity: TableEntry<Identity>) {
        self.users
            .delete_entry(&identity.partition_key, &identity.row_key, identity.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete user {:?}: {}", identity, e));
    }

    pub async fn is_user_name_available(&self, name: &str) -> Result<bool, IdentityError> {
        Ok(self
            .indices
            .get_entry::<EmptyEntry>(&NameIndexEntry::generate_partion_key(&name), name)
            .await?
            .is_none())
    }

    async fn insert_name_index(&self, identity: &IdentityEntry) -> Result<NameIndexEntry, IdentityError> {
        let name_index = NameIndexEntry::from_identity(identity);
        match self.indices.insert_entry(name_index.into_entry()).await {
            Ok(name_index) => Ok(NameIndexEntry::from_entry(name_index)),
            Err(e) => {
                if is_conflict(&e) {
                    Err(IdentityError::NameTaken)
                } else {
                    Err(IdentityError::from(e))
                }
            }
        }
    }

    pub async fn is_email_available(&self, email: &str) -> Result<bool, IdentityError> {
        Ok(self
            .indices
            .get_entry::<EmptyEntry>(&EmailIndexEntry::generate_partion_key(&email), &email)
            .await?
            .is_none())
    }

    async fn insert_email_index(&self, identity: &IdentityEntry) -> Result<Option<EmailIndexEntry>, IdentityError> {
        if let Some(email_index) = EmailIndexEntry::from_identity(&identity) {
            match self.indices.insert_entry(email_index.into_entry()).await {
                Ok(email_index) => Ok(Some(EmailIndexEntry::from_entry(email_index))),
                Err(e) => {
                    if is_conflict(&e) {
                        Err(IdentityError::EmailTaken)
                    } else {
                        Err(IdentityError::from(e))
                    }
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn create_user(
        &self,
        name: String,
        email: Option<String>,
        password: String,
    ) -> Result<IdentityEntry, IdentityError> {
        // validate input
        if !validate_username(&name) {
            log::info!("Invalid user name: {}", name);
            return Err(IdentityError::InvalidName);
        }
        if let Some(ref email) = email {
            if !validate_email(email) {
                log::info!("Invalid email: {}", email);
                return Err(IdentityError::InvalidEmail);
            }
        }

        // preliminary db checks (reduce the number of rollbacks)
        if !self.is_user_name_available(&name).await? {
            log::info!("User name {} already taken", name);
            return Err(IdentityError::NameTaken);
        }
        if let Some(ref email) = email {
            if !self.is_email_available(email).await? {
                log::info!("Email {} already taken", email);
                return Err(IdentityError::EmailTaken);
            }
        }

        let identity = self.insert_user(&name, &password, email.as_deref()).await?;

        let name_index = match self.insert_name_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (name_index): {:?}, {:?}", identity, e);
                self.delete_user(identity.into_entry()).await;
                return Err(e);
            }
        };

        let email_index = match self.insert_email_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (email_index): {:?}, {:?}", identity, e);
                self.delete_user(identity.into_entry()).await;
                self.delete_index(name_index.into_entry()).await;
                return Err(e);
            }
        };

        log::info!("New user registered: {:?}", identity);
        log::debug!("Name index: {:?}", name_index);
        log::debug!("Email index: {:?}", email_index);
        Ok(identity)
    }

    pub async fn find_identity_by_name_email(
        &self,
        name_email: &str,
        password: Option<&str>,
    ) -> Result<IdentityEntry, IdentityError> {
        let query_name = format!(
            "PartitionKey eq '{}' and RowKey eq '{}'",
            NameIndexEntry::generate_partion_key(name_email),
            name_email
        );
        let query_email = format!(
            "PartitionKey eq '{}' and RowKey eq '{}'",
            EmailIndexEntry::generate_partion_key(name_email),
            name_email
        );
        let query = format!("(({}) or ({}))", query_name, query_email);
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));

        self.find_identity_by_index(&query, password).await
    }
}

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
            Err(err) if is_conflict(&err) => Err(IdentityError::SessionKeyConflict),
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
            Err(err) if is_conflict(&err) => Err(IdentityError::SessionKeyConflict),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn create_session(&self, identity: &IdentityEntry, site: SiteInfo) -> Result<SessionEntry, IdentityError> {
        let mut try_count = 3usize;
        //todo: use backoff::retry
        loop {
            try_count -= 1;

            let session = match self.try_insert_session(identity, &site).await {
                Ok(session) => session,
                Err(IdentityError::SessionKeyConflict) if try_count > 0 => {
                    log::info!(
                        "Retrying ({}) session key already used by user: {}",
                        try_count,
                        identity.user_id()
                    );
                    continue;
                }
                Err(err) => return Err(err),
            };

            let session_index = match self.try_insert_session_index(&session).await {
                Ok(index) => index,
                Err(IdentityError::SessionKeyConflict) if try_count > 0 => {
                    log::info!("Retrying ({}) session key already used", try_count);
                    self.delete_sesssion(session.into_entry()).await;
                    continue;
                }
                Err(err) => {
                    log::info!("Creating session failed: {:?}, {:?}", identity, err);
                    self.delete_session(session.into_entry()).await;
                    return Err(err);
                }
            };

            log::info!("New session: {:?}", session);
            log::debug!("Session index: {:?}", session_index);
            return Ok(session);
        }
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
            let partion_key = format!("{}", identity.id());
            let row_key = format!("session-{}", session_key);
            self.sessions
                .get_entry(&partion_key, &row_key)
                .await?
                .map(SessionEntry::from_entry)
                .ok_or(IdentityError::UserNotFound)?
        };

        Ok(identity, session)
    }
}

fn is_conflict(err: &AzureError) -> bool {
    if let AzureError::UnexpectedHTTPResult(ref res) = err {
        if res.status_code() == 409 {
            return true;
        }
    }
    false
}

fn validate_username(name: &str) -> bool {
    name.chars().all(char::is_alphanumeric)
}
