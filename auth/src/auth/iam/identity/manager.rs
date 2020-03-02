use crate::auth::iam::{
    identity::{
        CoreIdentity, CoreIdentityIndexedData, EncodedEmail, EncodedName, Identity, IdentityCategory, IndexEmail,
        IndexIdentity, IndexName, IndexSequence, UserIdentity,
    },
    IAMConfig, IAMError,
};
use argon2;
use azure_sdk_storage_table::{CloudTable, Continuation, TableClient};
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

#[derive(Clone)]
pub struct IdentityManager {
    password_pepper: String,
    identity_id_generator: IdSequence,
    db: CloudTable,
}

// Handling identites
impl IdentityManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let client = TableClient::new(&config.storage_account, &config.storage_account_key)?;
        let db = CloudTable::new(client, "identities");
        db.create_if_not_exists().await?;

        let identity_id_generator = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                starting_value: 1_000_000,
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            IdSequence::new(id_counter.clone(), "identityId").with_granularity(10)
        };

        Ok(IdentityManager {
            password_pepper: config.password_pepper.clone(),
            identity_id_generator,
            db,
        })
    }

    async fn remove_identity<T>(&self, identity: T)
    where
        T: Identity,
    {
        let identity = identity.into_entity();
        let (p, r) = (identity.partition_key.clone(), identity.row_key.clone());
        self.db
            .delete_entity(identity)
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete identity([{}]/[{}]): {}", p, r, e));
    }

    async fn find_identity_by_id<T>(&self, id: &str) -> Result<T, IAMError>
    where
        T: Identity,
    {
        let (p, r) = T::entity_keys(&id);
        let identity = self.db.get(&p, &r, None).await?;
        let identity = identity.map(T::from_entity).ok_or(IAMError::IdentityNotFound)?;

        Ok(identity)
    }

    async fn remove_index<T>(&self, index: T)
    where
        T: IndexIdentity,
    {
        let index = index.into_entity();
        self.db
            .delete_entity(index)
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }

    async fn find_identity_by_index<T>(&self, query: &str) -> Result<T, IAMError>
    where
        T: Identity,
    {
        if let Some(indices) = self
            .db
            .execute_query::<CoreIdentityIndexedData>(Some(&query), &mut Continuation::start())
            .await?
        {
            match &indices[..] {
                [index] => {
                    let identity_id = &index.payload.identity_id;
                    let (p, r) = T::entity_keys(&identity_id);
                    let identity = self.db.get(&p, &r, None).await?.ok_or(IAMError::IdentityNotFound)?;
                    Ok(T::from_entity(identity))
                }
                _ => Err(IAMError::IdentityNotFound),
            }
        } else {
            Err(IAMError::IdentityNotFound)
        }
    }

    async fn find_user_by_index(&self, query: &str, password: Option<&str>) -> Result<UserIdentity, IAMError> {
        let identity = self.find_identity_by_index::<UserIdentity>(query).await?;

        if let Some(password) = password {
            // check password if provided
            if !argon2::verify_encoded(&identity.data().password_hash, password.as_bytes())
                .map_err(|err| IAMError::Internal(format!("Argon2 password validation failed: {}", err)))?
            {
                return Err(IAMError::PasswordNotMatching);
            }
        }

        Ok(identity)
    }

    async fn insert_sequence_index<T>(&self, identity: &T) -> Result<IndexSequence, IAMError>
    where
        T: Identity,
    {
        let sequence_index = IndexSequence::from_identity(identity);
        match self.db.insert_entity(sequence_index.into_entity()).await {
            Ok(sequence_index) => Ok(IndexSequence::from_entity(sequence_index)),
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
    pub async fn is_name_available(&self, name: &EncodedName) -> Result<bool, IAMError> {
        let (partition_key, row_key) = IndexName::entity_keys(name);
        Ok(self
            .db
            .get::<EmptyData>(&partition_key, &row_key, None)
            .await?
            .is_none())
    }

    async fn insert_name_index<T>(&self, identity: &T) -> Result<IndexName, IAMError>
    where
        T: Identity,
    {
        let name_index = IndexName::from_identity(identity);
        match self.db.insert_entity(name_index.into_entity()).await {
            Ok(name_index) => Ok(IndexName::from_entity(name_index)),
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
    pub async fn is_email_available(&self, cat: IdentityCategory, email: &EncodedEmail) -> Result<bool, IAMError> {
        let (partition_key, row_key) = IndexEmail::entity_keys(cat, email);
        Ok(self
            .db
            .get::<EmptyData>(&partition_key, &row_key, None)
            .await?
            .is_none())
    }

    async fn insert_email_index<T>(&self, identity: &T) -> Result<Option<IndexEmail>, IAMError>
    where
        T: Identity,
    {
        if let Some(email_index) = IndexEmail::from_identity(identity) {
            match self.db.insert_entity(email_index.into_entity()).await {
                Ok(email_index) => Ok(Some(IndexEmail::from_entity(email_index))),
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
        name: &EncodedName,
        password: &str,
        email: Option<&EncodedEmail>,
    ) -> Result<UserIdentity, BackoffError<IAMError>> {
        let mut rng = rand::thread_rng();
        let salt = String::from_utf8(
            SALT_ABC
                .choose_multiple(&mut rng, MAX_SALT_LEN)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let id = String::from_utf8(ID_ABC.choose_multiple(&mut rng, ID_LEN).cloned().collect::<Vec<_>>()).unwrap();
        let password_config = argon2::Config::default();
        let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &password_config)
            .map_err(|err| IAMError::Internal(format!("Argon2 password creation failed: {}", err)))
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
            .insert_entity(identity.into_entity())
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
    pub async fn create_user(
        &self,
        raw_name: &str,
        raw_email: Option<&str>,
        password: &str,
    ) -> Result<UserIdentity, IAMError> {
        let name = EncodedName::from_raw(raw_name)?;
        let email = if let Some(email) = raw_email {
            Some(EncodedEmail::from_raw(email)?)
        } else {
            None
        };

        // preliminary db checks (reduce the number of rollbacks)
        if !self.is_name_available(&name).await? {
            log::info!("User name {} already taken", raw_name);
            return Err(IAMError::NameTaken);
        }
        if let Some(ref email) = email {
            if !self.is_email_available(IdentityCategory::User, email).await? {
                log::info!("Email {} already taken", raw_email.unwrap_or(""));
                return Err(IAMError::EmailTaken);
            }
        }

        let identity = {
            let sequence_id = self.identity_id_generator.get().await?;
            backoff::Exponential::new(3, Duration::from_micros(10))
                .async_execute(|_| self.try_create_user_identity(sequence_id, &name, password, email.as_ref()))
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
    /// If password is given, the its validaity is also ensured
    pub async fn find_user_by_name_email(
        &self,
        raw_name_email: &str,
        password: Option<&str>,
    ) -> Result<UserIdentity, IAMError> {
        let query_name = {
            let (p, r) = IndexName::entity_keys(&EncodedName::from_raw(raw_name_email)?);
            format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r)
        };
        let query_email = {
            let (p, r) = IndexEmail::entity_keys(IdentityCategory::User, &EncodedEmail::from_raw(raw_name_email)?);
            format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r)
        };
        let query = format!("(({}) or ({}))", query_name, query_email);
        let query = format!(
            "$filter={}",
            utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC)
        );

        self.find_user_by_index(&query, password).await
    }

    /// Find a core identity by the id
    pub async fn find_core_identity_by_id(&self, id: &str) -> Result<CoreIdentity, IAMError> {
        self.find_identity_by_id::<CoreIdentity>(id).await
    }

    /// Find a user identity by the id
    pub async fn find_user_by_id(&self, id: &str) -> Result<UserIdentity, IAMError> {
        self.find_identity_by_id(id).await
    }
}
