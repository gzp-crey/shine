use super::error::IdentityError;
use super::IdentityConfig;
use crate::session::UserId;
use azure_sdk_core::errors::AzureError;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use futures::compat::Future01CompatExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn ignore_409(err: AzureError) -> Result<(), AzureError> {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 409 => Ok(()),
        e => Err(e),
    }
}

#[derive(Serialize, Deserialize)]
pub enum State {
    Active,
    Disabled,
}

#[derive(Serialize, Deserialize)]
pub struct IdentityUser {
    id: String,
    state: State,
    name: String,
    email: Option<String>,
    email_validate: bool,
    password_hash: String,
    roles: Vec<String>,
}

impl From<IdentityUser> for UserId {
    fn from(user: IdentityUser) -> Self {
        UserId::new(user.id, user.name, user.roles)
    }
}

pub struct IdentityDB {
    password_pepper: String,

    users: TableStorage,
    email_index: TableStorage,
    name_index: TableStorage,
}

impl IdentityDB {
    pub async fn new(config: &IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let email_index = TableStorage::new(table_service.clone(), "usersemailIndex");
        let name_index = TableStorage::new(table_service.clone(), "usersnameIndex");

        users.create_table().compat().await.or_else(ignore_409)?;
        email_index.create_table().compat().await.or_else(ignore_409)?;
        name_index.create_table().compat().await.or_else(ignore_409)?;

        Ok(IdentityDB {
            password_pepper: config.password_pepper.clone(),
            users,
            email_index,
            name_index,
        })
    }

    pub async fn create(&self, name: String, email: Option<String>, password: String) -> Result<IdentityUser, IdentityError> {
        // create identity
        let id = Uuid::new_v4().to_string();
        log::info!("Creating new user: {}", id);
        let password_hash = password;

        let mut user = IdentityUser {
            id,
            state: State::Active,
            name,
            email,
            email_validate: false,
            password_hash,
            roles: vec![],
        };

        self.users.insert_entity(&user).compat().await?;

        // create nameindex and ensure uniquiness
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
