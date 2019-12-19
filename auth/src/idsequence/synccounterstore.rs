use super::IdSequenceError;
use crate::azure_utils::ignore_409;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use core::ops::Range;
use serde::{Deserialize, Serialize};

pub struct SyncCounterConfig {
    storage_account: String,
    storage_account_key: String,
    table_name: String,
}

type CounterEntry = TableEntry<u64>;

pub struct SyncCounterStore {
    counters: TableStorage,
}

impl SyncCounterStore {
    pub async fn new(config: SyncCounterConfig) -> Result<Self, IdSequenceError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let counters = TableStorage::new(table_service.clone(), config.table_name);

        counters.create_table().await.or_else(ignore_409)?;

        Ok(SyncCounterStore { counters })
    }

    pub async fn get_range(&self, sequince_id: &str, count: u64) -> Result<Range<u64>, IdSequenceError> {
        //self.counters.get
        let mut entry = self.counters.get_entry::<u64>("counter", sequince_id).await;
        entry.payload += count;
        self.counters.update_entry(&entry).await;

        unimplemented!()
    }

    pub async fn get(&self, sequince_id: &str) -> Result<u64, IdSequenceError> {
        let mut range = self.get_range(sequince_id, 1).await?;
        if let Some(id) = range.next() {
            Ok(id)
        } else {
            Err(IdSequenceError::SequenceEnded)
        }
    }
}
