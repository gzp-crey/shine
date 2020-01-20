use super::*;
use argon2;
use block_modes::{
    block_padding::Pkcs7,
    {BlockMode, Cbc},
};
use blowfish::Blowfish;
use data_encoding;
use percent_encoding::{self, utf8_percent_encode};
use rand::{distributions::Alphanumeric, Rng};
use validator::validate_email;

const MAX_SALT_LEN: usize = 32;
const CIPHER_IV_LEN: usize = 8;
type Cipher = Cbc<Blowfish, Pkcs7>;

const ID_BASE_ENCODE: data_encoding::Encoding = data_encoding::BASE32_NOPAD;

fn validate_username(name: &str) -> bool {
    name.chars().all(char::is_alphanumeric)
}

// Handling identites
impl IdentityManager {
    fn user_id_cipher(&self, salt: &str) -> Result<Cipher, IdentityError> {
        let cipher = Cipher::new_var(&self.user_id_secret, &salt.as_bytes()[0..CIPHER_IV_LEN])?;
        Ok(cipher)
    }

    fn generate_user_id(&self, sequence_id: u64) -> Result<(String, String, String), IdentityError> {
        let sequence_id = format!("{:0>10}", sequence_id);
        let mut rng = rand::thread_rng();
        let salt = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(MAX_SALT_LEN)
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
    ) -> Result<IdentityEntry, BackoffError<IdentityError>> {
        let (id, sequence_id, salt) = self
            .generate_user_id(sequence_id)
            .map_err(|err| BackoffError::Permanent(IdentityError::from(err)))?;
        let password_config = argon2::Config::default();
        let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &password_config)
            .map_err(|err| BackoffError::Permanent(IdentityError::from(err)))?;
        log::info!("Created new user id:{}, pwh:{}", id, password_hash);
        let identity = IdentityEntry::new(
            id,
            sequence_id,
            salt,
            name.to_owned(),
            email.map(|e| e.to_owned()),
            password_hash,
        );

        let identity = self.users.insert_entry(identity.into_entry()).await.map_err(|err| {
            if azure_utils::is_precodition_error(&err) {
                BackoffError::Transient(IdentityError::UserIdConflict)
            } else {
                BackoffError::Permanent(IdentityError::UserIdConflict)
            }
        })?;

        Ok(IdentityEntry::from_entry(identity))
    }

    async fn insert_user(&self, name: &str, password: &str, email: Option<&str>) -> Result<IdentityEntry, IdentityError> {
        let sequence_id = self.user_id_generator.get().await?;
        backoff::Exponential::new(3, Duration::from_micros(10))
            .async_execute(|_| self.try_insert_user(sequence_id, name, password, email))
            .await
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
                if azure_utils::is_precodition_error(&e) {
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
                Err(err) => {
                    if azure_utils::is_precodition_error(&err) {
                        Err(IdentityError::EmailTaken)
                    } else {
                        Err(IdentityError::from(err))
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
