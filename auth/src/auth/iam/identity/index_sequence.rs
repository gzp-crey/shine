use super::{CoreIdentityIndexedData, Identity, IndexIdentityData, IndexIdentityEntity};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to an identity index by the sequence id
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IndexSequenceData {
    #[serde(flatten)]
    pub indexed_id: CoreIdentityIndexedData,
}

impl IndexIdentityData for IndexSequenceData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

/// Index identity by the sequence id
pub type IndexSequence = IndexIdentityEntity<IndexSequenceData>;

impl IndexSequence {
    pub fn entity_keys(sequence_id: u64) -> (String, String) {
        (format!("x_seq-{}", sequence_id % 100), sequence_id.to_string())
    }

    pub fn from_identity<T>(identity: &T) -> Self
    where
        T: Identity,
    {
        let core = identity.core();
        let sequence_id = core.sequence_id;
        let (partition_key, row_key) = Self::entity_keys(sequence_id);
        Self(TableEntity {
            partition_key,
            row_key,
            etag: None,
            timestamp: None,
            payload: IndexSequenceData {
                indexed_id: CoreIdentityIndexedData {
                    identity_id: core.id.clone(),
                },
            },
        })
    }
}
