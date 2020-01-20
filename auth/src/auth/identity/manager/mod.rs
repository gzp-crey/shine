use super::{
    error::IdentityError,
    identityentry::{EmailIndexEntry, EmptyEntry, Identity, IdentityEntry, IdentityIndex, NameIndexEntry},
    sessionentry::{Session, SessionEntry, SessionIndexEntry},
    IdentityConfig,
};
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use shine_core::{
    azure_utils,
    backoff::{self, Backoff, BackoffError},
    idgenerator::{IdSequence, SyncCounterConfig, SyncCounterStore},
    siteinfo::SiteInfo,
};
use std::{iter, str, time::Duration};

#[derive(Clone)]
pub struct IdentityManager {
    password_pepper: String,

    user_id_secret: Vec<u8>,
    user_id_generator: IdSequence,

    users: TableStorage,
    indices: TableStorage,
    sessions: TableStorage,
}

impl IdentityManager {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let users = TableStorage::new(table_service.clone(), "users");
        let indices = TableStorage::new(table_service.clone(), "userIndices");
        let sessions = TableStorage::new(table_service.clone(), "userSessions");

        indices.create_if_not_exists().await?;
        users.create_if_not_exists().await?;
        sessions.create_if_not_exists().await?;

        let user_id_generator = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            IdSequence::new(id_counter.clone(), "userid").with_granularity(10)
        };
        let user_id_secret = data_encoding::BASE64.decode(config.user_id_secret.as_bytes())?;

        Ok(IdentityManager {
            password_pepper: config.password_pepper.clone(),
            user_id_secret,
            users,
            indices,
            sessions,
            user_id_generator,
        })
    }

    async fn delete_index<K>(&self, index: TableEntry<K>) {
        self.indices
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }

    async fn find_identity_by_index(&self, query: &str, password: Option<&str>) -> Result<IdentityEntry, IdentityError> {
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
}

mod session;
mod user;
