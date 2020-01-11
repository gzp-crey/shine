use super::{
    error::IdentityError,
    identityentry::{EmailIndexEntry, EmptyEntry, Identity, IdentityEntry, IdentityIndex, NameIndexEntry},
    loginentry::{Login, LoginEntry, LoginIndexEntry},
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
use block_modes::{
    block_padding::Pkcs7,
    {BlockMode, Cbc},
};
use blowfish::Blowfish;
use data_encoding::BASE64;
use itertools::Itertools;
use percent_encoding::{self, utf8_percent_encode};
use rand::{distributions::Alphanumeric, Rng};
use std::{iter, str};
use validator::validate_email;

const SALT_LEN: usize = 32;
const CIPHER_IV_LEN: usize = 8;
const LOGIN_KEY_LEN: usize = 32;
type Cipher = Cbc<Blowfish, Pkcs7>;

const ID_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE32_NOPAD;
const KEY_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE64URL_NOPAD;

#[derive(Clone)]
pub struct IdentityDB {
    password_pepper: String,

    user_id_secret: Vec<u8>,
    user_id_generator: IdSequence,

    login_key_secret: Vec<u8>,
    login_key_generator: IdSequence,

    users: TableStorage,
    indices: TableStorage,
    logins: TableStorage,
}

// Handling identites
impl IdentityDB {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let indices = TableStorage::new(table_service.clone(), "userIndices");
        let logins = TableStorage::new(table_service.clone(), "userLogins");

        indices.create_if_not_exists().await?;
        users.create_if_not_exists().await?;
        logins.create_if_not_exists().await?;

        let (user_id_generator, login_key_generator) = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            (
                IdSequence::new(id_counter.clone(), "userid").with_granularity(10),
                IdSequence::new(id_counter.clone(), "loginkey").with_granularity(100),
            )
        };
        let user_id_secret = BASE64.decode(config.user_id_secret.as_bytes())?;
        let login_key_secret = BASE64.decode(config.login_key_secret.as_bytes())?;

        Ok(IdentityDB {
            password_pepper: config.password_pepper.clone(),
            user_id_secret,
            login_key_secret,
            users,
            indices,
            logins,
            user_id_generator,
            login_key_generator,
        })
    }

    fn user_id_cipher(&self, salt: &str) -> Result<Cipher, IdentityError> {
        let cipher = Cipher::new_var(&self.user_id_secret, &salt.as_bytes()[0..CIPHER_IV_LEN])?;
        Ok(cipher)
    }

    async fn generate_user_id(&self) -> Result<(String, String, String), IdentityError> {
        let sequence_id = format!("{:0>10}", self.user_id_generator.get().await?);
        let salt = {
            let mut rng = rand::thread_rng();
            iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(SALT_LEN)
                .collect::<String>()
        };

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

    async fn insert_user(&self, identity: IdentityEntry) -> Result<IdentityEntry, IdentityError> {
        match self.users.insert_entry(identity.into_entry()).await {
            Ok(identity) => Ok(IdentityEntry::from_entry(identity)),
            Err(e) => return Err(IdentityError::from(e)),
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

    async fn delete_index<K>(&self, index: TableEntry<K>) {
        self.indices
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
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
            if !argon2::verify_encoded(&identity.data().password_hash, password.as_bytes())? {
                return Err(IdentityError::PasswordNotMatching);
            }
        }

        Ok(identity)
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

        let identity = {
            let (id, sequence_id, salt) = self.generate_user_id().await?;
            let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &argon2::Config::default())?;
            log::info!("Created new user id:{}, pwh:{}", id, password_hash);
            let identity = IdentityEntry::new(id, sequence_id, salt, name, email, password_hash);
            self.insert_user(identity).await?
        };

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

// Login handling
impl IdentityDB {
    fn login_key_cipher(&self, salt: &str) -> Result<Cipher, IdentityError> {
        let cipher = Cipher::new_var(&self.login_key_secret, &salt.as_bytes()[0..CIPHER_IV_LEN])?;
        Ok(cipher)
    }

    async fn genrate_login_key(&self, salt: &str) -> Result<String, IdentityError> {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let mut rng = rand::thread_rng();
        let salt_sequence = iter::repeat(()).map(|()| CHARSET[rng.gen_range(0, CHARSET.len())] as char);
        let key_sequence = self.login_key_generator.get() .await?.to_string();
        let key_sequence = key_sequence.chars();
        let key_sequence = key_sequence.interleave(salt_sequence).take(LOGIN_KEY_LEN).collect::<String>();
        log::info!("key_sequence: {}", key_sequence);
        let cipher = self.login_key_cipher(&salt)?;
        let key = cipher.encrypt_vec(key_sequence.as_bytes());
        let key = KEY_BASE_ENCODE.encode(&key);

        debug_assert_eq!(self.decode_login_key(&key, &salt).ok(), Some(key_sequence));
        Ok(key)
    }

    fn decode_login_key(&self, key: &str, salt: &str) -> Result<String, IdentityError> {
        let key = KEY_BASE_ENCODE.decode(key.as_bytes())?;
        let cipher = self.login_key_cipher(salt)?;
        let key = cipher.decrypt_vec(&key)?;
        let key_sequence = str::from_utf8(&key)?.to_string();
        log::info!("Decoded login key:[{}]", key_sequence);
        Ok(key_sequence)
    }

    async fn insert_login(&self, identity: &IdentityEntry, site: SiteInfo) -> Result<LoginEntry, IdentityError> {
        let user_id = identity.user_id();
        let salt = &identity.data().salt;
        loop {
            let key = self.genrate_login_key(salt).await?;
            log::info!("Created new login key [{}] for {}", key, user_id);

            let login_session = LoginEntry::new(user_id.to_owned(), key, site.clone());
            match self.logins.insert_entry(login_session.into_entry()).await {
                Ok(session) => return Ok(LoginEntry::from_entry(session)),
                Err(err) if !is_conflict(&err) => return Err(err.into()),
                _ => log::warn!("Key collision with salt {}", salt),
            }
        }
    }

    async fn delete_login(&self, login: TableEntry<Login>) {
        self.users
            .delete_entry(&login.partition_key, &login.row_key, login.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete login {:?}: {}", login, e));
    }

    async fn insert_login_index(&self, login: &LoginEntry) -> Result<LoginIndexEntry, IdentityError> {
        let login_index = LoginIndexEntry::from_identity(login);
        match self.indices.insert_entry(login_index.into_entry()).await {
            Ok(login_index) => Ok(LoginIndexEntry::from_entry(login_index)),
            Err(e) => {
                if is_conflict(&e) {
                    Err(IdentityError::LoginKeyConflict)
                } else {
                    Err(IdentityError::from(e))
                }
            }
        }
    }

    pub async fn create_login(&self, identity: &IdentityEntry, site: SiteInfo) -> Result<LoginEntry, IdentityError> {
        let login = self.insert_login(identity, site).await?;

        let login_index = match self.insert_login_index(&login).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating login failed: {:?}, {:?}", identity, e);
                self.delete_login(login.into_entry()).await;
                return Err(e);
            }
        };

        log::info!("New login: {:?}", login);
        log::debug!("Login index: {:?}", login_index);
        Ok(login)
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
