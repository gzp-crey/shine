use super::error::IdentityError;
use super::identity::{EmailIndexEntry, IdentityEntry, NameIndexEntry, EmptyEntry};
use super::IdentityConfig;
use argon2;
use azure_sdk_core::errors::AzureError;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use azure_utils::idgenerator::{SaltedIdSequence, SyncCounterConfig, SyncCounterStore};

#[derive(Clone)]
pub struct IdentityDB {
    password_pepper: String,
    users: TableStorage,
    indices: TableStorage,
    id_generator: SaltedIdSequence,
}

impl IdentityDB {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let indices = TableStorage::new(table_service.clone(), "userIndices");

        let id_config = SyncCounterConfig {
            storage_account: config.storage_account.clone(),
            storage_account_key: config.storage_account_key.clone(),
            table_name: "idcounter".to_string(),
        };
        let id_counter = SyncCounterStore::new(id_config).await?;
        let id_generator = SaltedIdSequence::new(id_counter, "userid").with_granularity(10);

        users.create_if_not_exists().await?;
        indices.create_if_not_exists().await?;

        Ok(IdentityDB {
            password_pepper: config.password_pepper.clone(),
            users,
            indices,
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

    pub async fn create(&self, name: String, email: Option<String>, password: String) -> Result<IdentityEntry, IdentityError> {
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

        let id = self.id_generator.get().await?;
        log::info!("Creating new user: {}", id);

        let salt = id.split("-").skip(1).next().unwrap();
        let salt = format!("{}{}", salt, self.password_pepper);
        let argon2_config = argon2::Config::default();
        let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &argon2_config)?;

        let user = IdentityEntry::new(id, name, email, password_hash);
        let user = match self.users.insert_entry(user.into_entry()).await {
            Ok(user) => IdentityEntry::from_entry(user),
            Err(e) => return Err(IdentityError::from(e)),
        };

        let name_index = NameIndexEntry::from_identity(&user);
        let name_index = match self.indices.insert_entry(name_index.into_entry()).await {
            Ok(name_index) => NameIndexEntry::from_entry(name_index),
            Err(e) => {
                log::info!("Creating user failed (name_index): {:?}, {:?}", user, e);
                let _ = self
                    .users
                    .delete_entry(
                        &user.entry().partition_key,
                        &user.entry().row_key,
                        user.entry().etag.as_deref(),
                    )
                    .await;
                if is_conflict(&e) {
                    return Err(IdentityError::NameTaken);
                } else {
                    return Err(IdentityError::from(e));
                }
            }
        };

        let email_index = if let Some(email_index) = EmailIndexEntry::from_identity(&user) {
            let email_index = match self.indices.insert_entry(email_index.into_entry()).await {
                Ok(email_index) => EmailIndexEntry::from_entry(email_index),
                Err(e) => {
                    log::info!("Creating user failed (email_index): {:?}, {:?}", user, e);
                    let _ = self
                        .users
                        .delete_entry(
                            &user.entry().partition_key,
                            &user.entry().row_key,
                            user.entry().etag.as_deref(),
                        )
                        .await;
                    let _ = self
                        .indices
                        .delete_entry(
                            &name_index.entry().partition_key,
                            &name_index.entry().row_key,
                            name_index.entry().etag.as_deref(),
                        )
                        .await;

                    if is_conflict(&e) {
                        return Err(IdentityError::EmailTaken);
                    } else {
                        return Err(IdentityError::from(e));
                    }
                }
            };
            Some(email_index)
        } else {
            None
        };

        log::info!("New user registered: {:?}", user);
        log::debug!("Name index: {:?}", name_index);
        log::debug!("Email index: {:?}", email_index);
        Ok(user)
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
