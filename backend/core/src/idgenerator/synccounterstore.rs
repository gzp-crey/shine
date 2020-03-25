use super::IdSequenceError;
use crate::backoff::{self, Backoff, BackoffError};
use azure_sdk_storage_table::{CloudTable, TableClient};
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

    /// The initial value of the counter. The retuned id is not less
    /// then this value both for new and running seqeunces.
    pub starting_value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Counter {
    value: u64,
}

struct Inner {
    starting_value: u64,
    counters: CloudTable,
}

#[derive(Clone)]
pub struct SyncCounterStore(Arc<Inner>);

impl SyncCounterStore {
    pub async fn new(config: SyncCounterConfig) -> Result<Self, IdSequenceError> {
        let client = TableClient::new(&config.storage_account, &config.storage_account_key)?;
        let counters = CloudTable::new(client, config.table_name);

        counters.create_if_not_exists().await?;

        Ok(SyncCounterStore(Arc::new(Inner {
            starting_value: config.starting_value,
            counters,
        })))
    }

    async fn get_range_step(&self, sequence_id: &str, count: u64) -> Result<Range<u64>, BackoffError<IdSequenceError>> {
        match self
            .0
            .counters
            .get::<Counter>(PARTITION_KEY, sequence_id, None)
            .await
            .map_err(|err| BackoffError::Permanent(IdSequenceError::from(err)))?
        {
            None => {
                let start = self.0.starting_value;
                self.0
                    .counters
                    .insert(PARTITION_KEY, sequence_id, Counter { value: start + count })
                    .await
                    .map_err(|err| BackoffError::Permanent(IdSequenceError::from(err)))
                    .map(|ok| start..(ok.payload.value))
            }
            Some(mut entity) => {
                //ensure counter is larger than the requested initial
                let start = if entity.payload.value < self.0.starting_value {
                    self.0.starting_value
                } else {
                    entity.payload.value
                };
                entity.payload.value = start + count;
                self.0
                    .counters
                    .update_entity(entity)
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
