use super::error::IdentityError;
use super::identity::{EmailIndexEntry, IdentityEntry, NameIndexEntry};
use super::IdentityConfig;
use crate::session::UserId;
use azure_sdk_core::errors::AzureError;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct IdentityDB {
    password_pepper: String,
    users: TableStorage,
    email_index: TableStorage,
    name_index: TableStorage,
    //id_generator: IdGenerator,
}

impl IdentityDB {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let email_index = TableStorage::new(table_service.clone(), "usersemailIndex");
        let name_index = TableStorage::new(table_service.clone(), "usersnameIndex");

        users.create_if_not_exists().await?;
        email_index.create_if_not_exists().await?;
        name_index.create_if_not_exists().await?;

        Ok(IdentityDB {
            password_pepper: config.password_pepper.clone(),
            users,
            email_index,
            name_index,
        })
    }

    pub async fn create(&self, name: String, email: Option<String>, password: String) -> Result<IdentityEntry, IdentityError> {
        // create identity
        let id = Uuid::new_v4().to_string();
        log::info!("Creating new user: {}", id);
        let password_hash = password;

        let user = IdentityEntry::new(id, name, email, password_hash);
        let name_index = NameIndexEntry::from_identity(&user);
        let email_index = EmailIndexEntry::from_identity(&user);

        //let email_index = EmailIndexEntry::from_entry(self.email_index.insert_entry(&email_index.into_entry()).await?);
        //let name_index = NameIndexEntry::from_entry(self.name_index.insert_entry(&name_index.into_entry()).await?);
        // create user
        //let user = IdentityEntry::from_entry(self.users.insert_entry(user.into_entry()).await?);
        //log::info!("res1: {:?}", &serde_json::to_string(&user.entry()));

        // create nameindex and ensure uniquiness
        unimplemented!()
        //Ok(user)
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
