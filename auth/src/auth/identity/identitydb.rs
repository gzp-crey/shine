use super::error::IdentityError;
use super::identity::{EmailIndexEntry, IdentityEntry, NameIndexEntry};
use super::IdentityConfig;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use azure_utils::idgenerator::{SaltedIdSequence, SyncCounterConfig, SyncCounterStore};

#[derive(Clone)]
pub struct IdentityDB {
    password_pepper: String,
    users: TableStorage,
    email_index: TableStorage,
    name_index: TableStorage,
    id_generator: SaltedIdSequence,
}

impl IdentityDB {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let email_index = TableStorage::new(table_service.clone(), "usersemailIndex");
        let name_index = TableStorage::new(table_service.clone(), "usersnameIndex");

        let id_config = SyncCounterConfig {
            storage_account: config.storage_account.clone(),
            storage_account_key: config.storage_account_key.clone(),
            table_name: "idcounter".to_string(),
        };
        let id_counter = SyncCounterStore::new(id_config).await?;
        let id_generator = SaltedIdSequence::new(id_counter, "userid").with_granularity(10);

        users.create_if_not_exists().await?;
        email_index.create_if_not_exists().await?;
        name_index.create_if_not_exists().await?;

        Ok(IdentityDB {
            password_pepper: config.password_pepper.clone(),
            users,
            email_index,
            name_index,
            id_generator,
        })
    }

    pub async fn create(&self, name: String, email: Option<String>, password: String) -> Result<IdentityEntry, IdentityError> {
        let id = self.id_generator.get().await?;
        log::info!("Creating new user: {}", id);
        let password_hash = password;

        let user = IdentityEntry::new(id, name, email, password_hash);
        let user = match self.users.insert_entry(user.into_entry()).await {
            Ok(user) => IdentityEntry::from_entry(user),
            Err(e) => return Err(IdentityError::from(e)),
        };

        let name_index = NameIndexEntry::from_identity(&user);
        let name_index = match self.name_index.insert_entry(name_index.into_entry()).await {
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
                return Err(IdentityError::from(e));
            }
        };

        if let Some(email_index) = EmailIndexEntry::from_identity(&user) {
            let email_index = match self.email_index.insert_entry(email_index.into_entry()).await {
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
                        .name_index
                        .delete_entry(
                            &name_index.entry().partition_key,
                            &name_index.entry().row_key,
                            name_index.entry().etag.as_deref(),
                        )
                        .await;
                    return Err(IdentityError::from(e));
                }
            };
        }

        log::info!("New user registered: {:?}", user);
        Ok(user)
    }
}

/*
struct NameIndex {
    id: String,
    name: String,
}

struct EmailIndex {
    id: String,
    name: String,
}

pub struct IdentityDB {
    identities: TableStorage,
    nameindex: TableStorage,
    emailIndex: TableStorage,
}

impl IdentityDB {}
    pub fn disable(name: String, email: Option<String>, password_hash: String) -> Result<Identity, Error> {

    }

    pub fn prune() -> Result<(), Error> {

    }

    pub fn validate_email(id: String) -> Result<Identity, Error> {

    }

    pub fn change_password(id:String, old_password_hash: String, new_password_hash:String) -> Result<Identity,()> {

    }

    pub fn find_by_name(name:String) -> Result<Identity, Error> {

    }

    pub fn find_by_email(name:String) -> Result<Identity, Error> {

    }

    pub fn find_by_id(name:String) -> Result<Identity, Error> {

    }

    pub fn set_role(id: String, roles:Vec<String>)-> Result<Identity, Error> {

    }

    pub fn add_role(id: String, roles:Vec<String>)-> Result<Identity, Error> {

    }

    pub fn remove_role(id: String, roles:Vec<String>)-> Result<Identity, Error> {

    }
}

*/
