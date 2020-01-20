use super::IdSequenceError;
use crate::backoff::{self, Backoff, BackoffError};
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use core::ops::Range;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

const PARTITION_KEY: &str = "counter";

#[derive(Debug, Clone)]
pub struct SyncCounterConfig {
    pub storage_account: String,
    pub storage_account_key: String,
    pub table_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Counter {
    value: u64,
}

struct Inner {
    counters: TableStorage,
}

#[derive(Clone)]
pub struct SyncCounterStore(Arc<Inner>);

impl SyncCounterStore {
    pub async fn new(config: SyncCounterConfig) -> Result<Self, IdSequenceError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let counters = TableStorage::new(table_service.clone(), config.table_name);

        counters.create_if_not_exists().await?;

        Ok(SyncCounterStore(Arc::new(Inner { counters })))
    }

    async fn get_range_step(&self, sequence_id: &str, count: u64) -> Result<Range<u64>, BackoffError<IdSequenceError>> {
        match self
            .0
            .counters
            .get_entry::<Counter>(PARTITION_KEY, sequence_id)
            .await
            .map_err(|err| BackoffError::Permanent(IdSequenceError::from(err)))?
        {
            None => {
                let entry = TableEntry {
                    partition_key: PARTITION_KEY.to_string(),
                    row_key: sequence_id.to_string(),
                    etag: None,
                    payload: Counter { value: count },
                };
                self.0
                    .counters
                    .insert_entry(entry)
                    .await
                    .map_err(|err| BackoffError::Permanent(IdSequenceError::from(err)))
                    .map(|ok| 0..(ok.payload.value))
            }
            Some(mut entry) => {
                let start = entry.payload.value;
                entry.payload.value += count;
                self.0
                    .counters
                    .update_entry(entry)
                    .await
                    .map_err(|err| BackoffError::Permanent(IdSequenceError::from(err)))
                    .map(|ok| start..(ok.payload.value))
            }
        }
    }

    pub async fn get_range(&self, sequence_id: &str, count: u64) -> Result<Range<u64>, IdSequenceError> {
        backoff::Exponential::new(10, Duration::from_micros(10))
            .async_execute(|_| self.get_range_step(sequence_id, count))
            .await
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
