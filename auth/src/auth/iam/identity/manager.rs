use crate::auth::iam::{
    identity::{EmailIndex, Identity, IdentityIndex, IdentityIndexedId, NameIndex, SequenceIndex, UserIdentity},
    IAMConfig, IAMError,
};
use argon2;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use percent_encoding::{self, utf8_percent_encode};
use rand::{self, seq::SliceRandom};
use shine_core::{
    azure_utils::{self, table_storage::EmptyData},
    backoff::{self, Backoff, BackoffError},
    idgenerator::{IdSequence, SyncCounterConfig, SyncCounterStore},
};
use std::{str, time::Duration};

const ID_LEN: usize = 8;
const ID_ABC: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

const MAX_SALT_LEN: usize = 32;
const SALT_ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn validate_username(name: &str) -> bool {
    name.chars().all(char::is_alphanumeric)
}

fn validate_email(email: &str) -> bool {
    validator::validate_email(email)
}

#[derive(Clone)]
pub struct IdentityManager {
    password_pepper: String,
    identity_id_generator: IdSequence,
    db: TableStorage,
}

// Handling identites
impl IdentityManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let identities_db = TableStorage::new(table_service.clone(), "identities");

        identities_db.create_if_not_exists().await?;

        let identity_id_generator = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            IdSequence::new(id_counter.clone(), "identityId").with_granularity(10)
        };

        Ok(IdentityManager {
            password_pepper: config.password_pepper.clone(),
            identity_id_generator,
            db: identities_db,
        })
    }

    async fn remove_index<T>(&self, index: T)
    where
        T: IdentityIndex,
    {
        let index = index.into_entity();
        self.db
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }

    async fn remove_identity<T>(&self, identity: T)
    where
        T: Identity,
    {
        let identity = identity.into_entity();
        self.db
            .delete_entry(&identity.partition_key, &identity.row_key, identity.etag.as_deref())
            .await
            .unwrap_or_else(|e| {
                log::error!(
                    "Failed to delete identity([{}]/[{}]): {}",
                    identity.partition_key,
                    identity.row_key,
                    e
                )
            });
    }

    async fn find_identity_by_index<T>(&self, query: &str) -> Result<T, IAMError>
    where
        T: Identity,
    {
        let mut index = self.db.query_entries::<IdentityIndexedId>(Some(&query)).await?;
        assert!(index.len() <= 1);
        let index = index.pop().ok_or(IAMError::IdentityNotFound)?;

        let identity_id = &index.payload.identity_id;
        let (p, r) = T::entity_keys(&identity_id);
        let identity = self.db.get_entry(&p, &r).await?;
        let identity = identity.map(T::from_entity).ok_or(IAMError::IdentityNotFound)?;

        Ok(identity)
    }

    async fn find_user_by_index(&self, query: &str, password: Option<&str>) -> Result<UserIdentity, IAMError> {
        let identity = self.find_identity_by_index::<UserIdentity>(query).await?;

        if let Some(password) = password {
            // check password if provided
            if !argon2::verify_encoded(&identity.data().password_hash, password.as_bytes())? {
                return Err(IAMError::PasswordNotMatching);
            }
        }

        Ok(identity)
    }

    async fn insert_sequence_index<T>(&self, identity: &T) -> Result<SequenceIndex, IAMError>
    where
        T: Identity,
    {
        let sequence_index = SequenceIndex::from_identity(identity);
        match self.db.insert_entry(sequence_index.into_entity()).await {
            Ok(sequence_index) => Ok(SequenceIndex::from_entity(sequence_index)),
            Err(e) => {
                if azure_utils::is_precodition_error(&e) {
                    Err(IAMError::SequenceIdTaken)
                } else {
                    Err(IAMError::from(e))
                }
            }
        }
    }

    /// Return if the given name can be used as a new identity name
    pub async fn is_name_available(&self, name: &str) -> Result<bool, IAMError> {
        let (partition_key, row_key) = NameIndex::entity_keys(name);
        Ok(self.db.get_entry::<EmptyData>(&partition_key, &row_key).await?.is_none())
    }

    async fn insert_name_index<T>(&self, identity: &T) -> Result<NameIndex, IAMError>
    where
        T: Identity,
    {
        let name_index = NameIndex::from_identity(identity);
        match self.db.insert_entry(name_index.into_entity()).await {
            Ok(name_index) => Ok(NameIndex::from_entity(name_index)),
            Err(e) => {
                if azure_utils::is_precodition_error(&e) {
                    Err(IAMError::NameTaken)
                } else {
                    Err(IAMError::from(e))
                }
            }
        }
    }

    /// Return if the given email can be used.
    pub async fn is_email_available(&self, email: &str) -> Result<bool, IAMError> {
        let (partition_key, row_key) = EmailIndex::entity_keys(email);
        Ok(self.db.get_entry::<EmptyData>(&partition_key, &row_key).await?.is_none())
    }

    async fn insert_email_index<T>(&self, identity: &T) -> Result<Option<EmailIndex>, IAMError>
    where
        T: Identity,
    {
        if let Some(email_index) = EmailIndex::from_identity(identity) {
            match self.db.insert_entry(email_index.into_entity()).await {
                Ok(email_index) => Ok(Some(EmailIndex::from_entity(email_index))),
                Err(err) => {
                    if azure_utils::is_precodition_error(&err) {
                        Err(IAMError::EmailTaken)
                    } else {
                        Err(IAMError::from(err))
                    }
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn try_create_user_identity(
        &self,
        sequence_id: u64,
        name: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<UserIdentity, BackoffError<IAMError>> {
        let mut rng = rand::thread_rng();
        let salt = String::from_utf8(SALT_ABC.choose_multiple(&mut rng, MAX_SALT_LEN).cloned().collect::<Vec<_>>()).unwrap();
        let id = String::from_utf8(ID_ABC.choose_multiple(&mut rng, ID_LEN).cloned().collect::<Vec<_>>()).unwrap();
        let password_config = argon2::Config::default();
        let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &password_config)
            .map_err(IAMError::from)
            .map_err(IAMError::into_backoff)?;

        log::info!("Created new user id:{}, pwh:{}", id, password_hash);
        let identity = UserIdentity::new(
            id,
            sequence_id,
            salt,
            name.to_owned(),
            email.map(|e| e.to_owned()),
            password_hash,
        );

        let identity = self
            .db
            .insert_entry(identity.into_entity())
            .await
            .map_err(|err| {
                if azure_utils::is_precodition_error(&err) {
                    IAMError::IdentityIdConflict
                } else {
                    IAMError::from(err)
                }
            })
            .map_err(IAMError::into_backoff)?;

        Ok(UserIdentity::from_entity(identity))
    }

    /// Creates a new user identity.
    pub async fn create_user(&self, name: String, email: Option<String>, password: String) -> Result<UserIdentity, IAMError> {
        // validate input
        if !validate_username(&name) {
            log::info!("Invalid user name: {}", name);
            return Err(IAMError::InvalidName);
        }
        if let Some(ref email) = email {
            if !validate_email(email) {
                log::info!("Invalid email: {}", email);
                return Err(IAMError::InvalidEmail);
            }
        }

        // preliminary db checks (reduce the number of rollbacks)
        if !self.is_name_available(&name).await? {
            log::info!("User name {} already taken", name);
            return Err(IAMError::NameTaken);
        }
        if let Some(ref email) = email {
            if !self.is_email_available(email).await? {
                log::info!("Email {} already taken", email);
                return Err(IAMError::EmailTaken);
            }
        }

        let identity = {
            let sequence_id = self.identity_id_generator.get().await?;
            backoff::Exponential::new(3, Duration::from_micros(10))
                .async_execute(|_| self.try_create_user_identity(sequence_id, &name, &password, email.as_deref()))
                .await?
        };

        let sequence_index = match self.insert_sequence_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (sequence_index): {:?}, {:?}", identity, e);
                self.remove_identity(identity).await;
                return Err(e);
            }
        };

        let name_index = match self.insert_name_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (name_index): {:?}, {:?}", identity, e);
                self.remove_identity(identity).await;
                self.remove_index(sequence_index).await;
                return Err(e);
            }
        };

        let email_index = match self.insert_email_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (email_index): {:?}, {:?}", identity, e);
                self.remove_identity(identity).await;
                self.remove_index(sequence_index).await;
                self.remove_index(name_index).await;
                return Err(e);
            }
        };

        log::info!("New user registered: {:?}", identity);
        log::debug!("Name index: {:?}", name_index);
        log::debug!("Email index: {:?}", email_index);
        log::debug!("Sequence index: {:?}", sequence_index);
        Ok(identity)
    }

    /// Find a user identity by email or name.
    /// If a password it is also checked.
    pub async fn find_user_by_name_email(&self, name_email: &str, password: Option<&str>) -> Result<UserIdentity, IAMError> {
        let query_name = {
            let (p, r) = NameIndex::entity_keys(name_email);
            format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r)
        };
        let query_email = {
            let (p, r) = EmailIndex::entity_keys(name_email);
            format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r)
        };
        let query = format!("(({}) or ({}))", query_name, query_email);
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));

        self.find_user_by_index(&query, password).await
    }
}
