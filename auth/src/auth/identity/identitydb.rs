use super::{
    error::IdentityError,
    identity::{EmailIndexEntry, EmptyEntry, Identity, IdentityEntry, IdentityIndex, NameIndexEntry},
    siteinfo::SiteInfo,
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
use data_encoding::BASE64;
use percent_encoding::{self, utf8_percent_encode};
use rand::{distributions::Alphanumeric, Rng};
use ring::aead;
use std::{iter, str};
use validator::validate_email;

#[derive(Clone)]
pub struct IdentityDB {
    password_pepper: String,
    user_id_key: Vec<u8>,
    login_key_key: Vec<u8>,
    users: TableStorage,
    indices: TableStorage,
    logins: TableStorage,
    id_generator: IdSequence,
}

static SALT_LEN: usize = 32;

static ID_ENCRYPT: &aead::Algorithm = &aead::AES_128_GCM;
static ID_BASE_ENCODE: &data_encoding::Encoding = &data_encoding::BASE32_NOPAD;

static LOGIN_KEY_ENCRYPT: &aead::Algorithm = &aead::CHACHA20_POLY1305;
static LOGIN_KEY_BASE_ENCODE: &data_encoding::Encoding = &data_encoding::BASE64URL_NOPAD;

impl IdentityDB {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let indices = TableStorage::new(table_service.clone(), "userIndices");
        let logins = TableStorage::new(table_service.clone(), "userLogins");

        let id_generator = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            IdSequence::new(id_counter, "userid").with_granularity(10)
        };
        let user_id_key = {
            let key = BASE64.decode(config.user_id_secret.as_bytes())?;
            let len = ID_ENCRYPT.key_len();
            if key.len() < len {
                return Err(IdentityError::Encryption(format!(
                    "user_id_secret is too short, required at least: {} bytes, got: {}",
                    len,
                    key.len()
                )));
            }
            key[0..len].to_vec()
        };
        let login_key_key = {
            let key = BASE64.decode(config.login_key_secret.as_bytes())?;
            let len = LOGIN_KEY_ENCRYPT.key_len();
            if key.len() < len {
                return Err(IdentityError::Encryption(format!(
                    "login_key_secret is too short, required at least: {} bytes, got: {}",
                    len,
                    key.len()
                )));
            }
            key[0..len].to_vec()
        };

        users.create_if_not_exists().await?;
        indices.create_if_not_exists().await?;

        Ok(IdentityDB {
            password_pepper: config.password_pepper.clone(),
            user_id_key,
            login_key_key,
            users,
            indices,
            logins,
            id_generator,
        })
    }

    pub async fn is_user_name_available(&self, name: &str) -> Result<bool, IdentityError> {
        Ok(self
            .indices
            .get_entry::<EmptyEntry>(&NameIndexEntry::generate_partion_key(&name), name)
            .await?
            .is_none())
    }

    pub async fn is_email_available(&self, email: &str) -> Result<bool, IdentityError> {
        Ok(self
            .indices
            .get_entry::<EmptyEntry>(&EmailIndexEntry::generate_partion_key(&email), &email)
            .await?
            .is_none())
    }

    async fn genrate_user_id(&self) -> Result<(String, String, String), IdentityError> {
        let sequence_id = self.id_generator.get().await?.to_string();
        let salt = {
            let mut rng = rand::thread_rng();
            iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(SALT_LEN)
                .collect::<String>()
        };

        let nonce = aead::Nonce::try_assume_unique_for_key(&salt.as_bytes()[0..aead::NONCE_LEN])?;
        let key = aead::UnboundKey::new(&ID_ENCRYPT, &self.user_id_key)?;
        let key = aead::LessSafeKey::new(key);
        let aad = aead::Aad::empty();
        let mut id = sequence_id.as_bytes().to_owned();
        key.seal_in_place_append_tag(nonce, aad, &mut id)?;

        let id = ID_BASE_ENCODE.encode(&id);

        log::info!("Created new user id:[{}], seq: {}, salt: {}", id, sequence_id, salt);
        Ok((id, sequence_id, salt))
    }

    fn decode_user_id(&self, id: &str, salt: &str) -> Result<String, IdentityError> {
        let mut id = ID_BASE_ENCODE.decode(id.as_bytes())?;

        let nonce = aead::Nonce::try_assume_unique_for_key(&salt.as_bytes()[0..aead::NONCE_LEN])?;
        let key = aead::UnboundKey::new(&ID_ENCRYPT, &self.user_id_key)?;
        let key = aead::LessSafeKey::new(key);
        let aad = aead::Aad::empty();
        let id = key.open_in_place(nonce, aad, &mut id)?;
        let id = str::from_utf8(&id)?;

        log::info!("Decoded user id:[{}]", id);

        Ok(id.to_string())
    }

    async fn delete_user(&self, identity: TableEntry<Identity>) {
        self.users
            .delete_entry(&identity.partition_key, &identity.row_key, identity.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("failed to delete user: {}", e));
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

    async fn delete_index<K>(&self, index: TableEntry<K>) {
        self.indices
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }

    pub async fn create(&self, name: String, email: Option<String>, password: String) -> Result<IdentityEntry, IdentityError> {
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

        // preliminary tests to avoid unecessary rollbacks
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

        // create user entity
        let (id, sequence_id, salt) = {
            let (id, sequence_id, salt) = self.genrate_user_id().await?;
            if self.decode_user_id(&id, &salt)? != sequence_id {
                return Err(IdentityError::Encryption(format!(
                    "User id encode-decode error, sequence_id: {}, salt: {}",
                    sequence_id, salt
                )));
            }
            (id, sequence_id, salt)
        };
        let password_hash = {
            let argon2_config = argon2::Config::default();
            argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &argon2_config)?
        };
        let identity = IdentityEntry::new(id, sequence_id, salt, name, email, password_hash);
        let identity = match self.users.insert_entry(identity.into_entry()).await {
            Ok(identity) => IdentityEntry::from_entry(identity),
            Err(e) => return Err(IdentityError::from(e)),
        };

        // create indices
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

    pub fn create_login_key(&self, identiy: &IdentityEntry, site: SiteInfo) -> Result<String, IdentityError> {
        unimplemented!()
    }

    async fn find_by_index(&self, query: &str, password: Option<&str>) -> Result<IdentityEntry, IdentityError> {
        let index = self.indices.query_entries::<IdentityIndex>(Some(&query)).await?;
        assert!(index.len() <= 1);
        let index = index.first().ok_or(IdentityError::UserNotFound)?;

        let user_id = &index.payload.user_id;
        let partion_key = IdentityEntry::generate_partion_key(&user_id);
        let identity = self.users.get_entry(&partion_key, &user_id).await?;
        let identity = identity.map(IdentityEntry::from_entry).ok_or(IdentityError::UserNotFound)?;

        if let Some(password) = password {
            // check password if provided, this is a low level function and it's ok if no password was
            if !argon2::verify_encoded(&identity.identity().password_hash, password.as_bytes())? {
                return Err(IdentityError::PasswordNotMatching);
            }
        }

        Ok(identity)
    }

    pub async fn find_by_login(&self, login: &str, password: Option<&str>) -> Result<IdentityEntry, IdentityError> {
        let query_name = format!(
            "PartitionKey eq '{}' and RowKey eq '{}'",
            NameIndexEntry::generate_partion_key(login),
            login
        );
        let query_email = format!(
            "PartitionKey eq '{}' and RowKey eq '{}'",
            EmailIndexEntry::generate_partion_key(login),
            login
        );
        let query = format!("(({}) or ({}))", query_name, query_email);
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));

        self.find_by_index(&query, password).await
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
