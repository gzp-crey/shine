use crate::idgenerator::{IdSequenceError, SyncCounterStore};
use futures::lock::Mutex;
use rand::{distributions::Alphanumeric, Rng};
use std::iter;
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone)]
pub struct SaltedIdSequence {
    name: String,
    granularity: u64,
    counter_store: SyncCounterStore,
    range: Arc<Mutex<Range<u64>>>,
}

impl SaltedIdSequence {
    pub fn new<S: Into<String>>(counter_store: SyncCounterStore, name: S) -> SaltedIdSequence {
        SaltedIdSequence {
            name: name.into(),
            granularity: 100,
            counter_store,
            range: Arc::new(Mutex::new(0u64..0u64)),
        }
    }

    pub fn with_granularity(self, granularity: u64) -> Self {
        SaltedIdSequence {
            granularity: granularity,
            ..self
        }
    }

    async fn get_id(&self) -> Result<u64, IdSequenceError> {
        let mut l = self.range.lock().await;
        if let Some(id) = l.next() {
            Ok(id)
        } else {
            *l = self.counter_store.get_range(&self.name, self.granularity).await?;
            Ok(l.next().unwrap())
        }
    }

    pub async fn get(&self) -> Result<String, IdSequenceError> {
        let id = self.get_id().await?;
        let mut rng = rand::thread_rng();
        let salt: String = iter::repeat(()).map(|()| rng.sample(Alphanumeric)).take(7).collect();
        Ok(format!("{}-{}", id, salt))
    }
}
