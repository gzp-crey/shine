use super::error::IdentityError;
use actix_web::{web::Json, HttpResponse};
use azure_sdk_core::errors::AzureError;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use futures::compat::Future01CompatExt;
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Serialize, Deserialize)]
pub struct IdentityConfig {
    storage_account: String,
    storage_account_key: String,
}

fn ignore_409(err: AzureError) -> Result<(), AzureError> {
    match err {
        AzureError::UnexpectedHTTPResult(e) if e.status_code() == 409 => Ok(()),
        e => Err(e),
    }
}

pub struct IdentityDB {
    users: TableStorage,
    email_index: TableStorage,
    name_index: TableStorage,
}

impl IdentityDB {
    pub fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let email_index = TableStorage::new(table_service.clone(), "usersemailIndex");
        let name_index = TableStorage::new(table_service.clone(), "usersnameIndex");        

        Ok (IdentityDB {
            users, email_index, name_index
        })
    }   

    pub fn init_tables(&self) -> impl Future<Output = Result<(), IdentityError>> {
        futures::join3(self.users.create_table().compat().await.or_else(ignore_409)?;
        //this.email_index.create_table().compat().await.or_else(ignore_409)?;
        //this.name_index.create_table().compat().await.or_else(ignore_409)?;
        //Ok(())
    }
}



pub async fn test_az(config: Json<IdentityConfig>) -> Result<HttpResponse, IdentityError> {
    let db = IdentityDB::new(config.into_inner())?;
    init_tables(&db).await?;
    Ok(HttpResponse::Ok().finish())
}

/*
pub enum State {
    Active,
    Disabled,
}

pub struct User {
    id : String,
    state: State,
    name: String,
    email: Option<String>,
    email_validate: bool,
    password_hash: String,
    roles: Vec<String>,
}

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
    pub fn new(config: Config) -> Self {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users_db = TableStorage::new(table_service.clone(), "users");
        let email_index = TableStorage::new(table_service.clone(), "usersemailIndex");
        let name_index = TableStorage::new(table_service.clone(), "usersnameIndex");

        users_db.create_table().compat().await.or_else(ignore_409)?;
        email_index.create_table().compat().await.or_else(ignore_409)?;
        name_index.create_table().compat().await.or_else(ignore_409)?;
    }

    pub fn create(name: String, email: Option<String>, password_hash: String) -> Result<Identity, Error> {
        // create identity
        // create nameindex (ensure uniquiness)
        // create emailindex (ensure uniquiness)
    }

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
