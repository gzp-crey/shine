use super::error::IdentityError;
use super::IdentityConfig;
use crate::session::UserId;
use azure_sdk_core::errors::AzureError;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn ignore_409(err: AzureError) -> Result<(), AzureError> {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 409 => Ok(()),
        e => Err(e),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum State {
    Active,
    Disabled,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityUser {
    partition_key: String,
    row_key: String,
    state: State,

    pub name: String,
    pub email: Option<String>,
    pub email_validate: bool,
    pub password_hash: String,
    //pub roles: Vec<String>,
}

impl IdentityUser {
    fn new(id: String, name: String, email: Option<String>, password_hash: String) -> IdentityUser {
        IdentityUser {
            partition_key: id.clone(),
            row_key: id.clone(),
            state: State::Active,
            name,
            email,
            email_validate: false,
            password_hash,
            //roles: vec![],
        }
    }

    pub fn id(&self) -> &str {
        &self.row_key
    }
}

impl From<IdentityUser> for UserId {
    fn from(user: IdentityUser) -> Self {
        UserId::new(user.row_key, user.name, vec![] /*user.roles*/)
    }
}

#[derive(Clone)]
pub struct IdentityDB {
    password_pepper: String,
    users: TableStorage,
    email_index: TableStorage,
    name_index: TableStorage,
}

impl IdentityDB {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let email_index = TableStorage::new(table_service.clone(), "usersemailIndex");
        let name_index = TableStorage::new(table_service.clone(), "usersnameIndex");

        users.create_table().await.or_else(ignore_409)?;
        email_index.create_table().await.or_else(ignore_409)?;
        name_index.create_table().await.or_else(ignore_409)?;

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

        let user = IdentityUser::new(id, name, email, password_hash);

        log::info!("res1: {:?}", &serde_json::to_string(&user));
        /*log::info!(
            "res: {:?}",
            self.users
                .get_entity::<std::collections::HashMap<String, String>>("hello", "world")
                .compat()
                .await
        );

        self.users.insert_entity(&user).compat().await?;*/

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
