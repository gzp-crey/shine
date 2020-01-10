use crate::idgenerator::{IdSequenceError, SyncCounterStore};
use futures::lock::Mutex;
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone)]
pub struct IdSequence {
    name: String,
    granularity: u64,
    counter_store: SyncCounterStore,
    range: Arc<Mutex<Range<u64>>>,
}

impl IdSequence {
    pub fn new<S: Into<String>>(counter_store: SyncCounterStore, name: S) -> IdSequence {
        IdSequence {
            name: name.into(),
            granularity: 100,
            counter_store,
            range: Arc::new(Mutex::new(0u64..0u64)),
        }
    }

    pub fn with_granularity(self, granularity: u64) -> Self {
        IdSequence {
            granularity: granularity,
            ..self
        }
    }

    pub async fn get(&self) -> Result<u64, IdSequenceError> {
        let mut l = self.range.lock().await;
        if let Some(id) = l.next() {
            Ok(id)
        } else {
            *l = self.counter_store.get_range(&self.name, self.granularity).await?;
            Ok(l.next().unwrap())
        }
    }
}
