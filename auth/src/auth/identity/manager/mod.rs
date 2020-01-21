use super::{error::IdentityError, IdentityConfig};
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
use std::{str, time::Duration};

#[derive(Clone)]
pub struct IdentityManager {
    password_pepper: String,

    identity_id_secret: Vec<u8>,
    identity_id_generator: IdSequence,

    identities: TableStorage,
    indices: TableStorage,
    sessions: TableStorage,
}

impl IdentityManager {
    pub async fn new(config: IdentityConfig) -> Result<Self, IdentityError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let identities = TableStorage::new(table_service.clone(), "identities");
        let indices = TableStorage::new(table_service.clone(), "identityIndices");
        let sessions = TableStorage::new(table_service.clone(), "identitySessions");

        indices.create_if_not_exists().await?;
        identities.create_if_not_exists().await?;
        sessions.create_if_not_exists().await?;

        let identity_id_generator = {
            let id_config = SyncCounterConfig {
                storage_account: config.storage_account.clone(),
                storage_account_key: config.storage_account_key.clone(),
                table_name: "idcounter".to_string(),
            };
            let id_counter = SyncCounterStore::new(id_config).await?;
            IdSequence::new(id_counter.clone(), "identityId").with_granularity(10)
        };
        let identity_id_secret = data_encoding::BASE64.decode(config.identity_id_secret.as_bytes())?;

        Ok(IdentityManager {
            password_pepper: config.password_pepper.clone(),
            identity_id_secret,
            identities,
            indices,
            sessions,
            identity_id_generator,
        })
    }

    async fn delete_index<K>(&self, index: TableEntry<K>) {
        self.indices
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }
}

mod identity;
//mod session;
