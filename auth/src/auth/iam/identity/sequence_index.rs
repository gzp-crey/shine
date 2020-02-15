use super::{Identity, IdentityIndex, IdentityIndexData, IdentityIndexedId};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to an identity index by the sequence id
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SequenceIndexData {
    #[serde(flatten)]
    pub indexed_id: IdentityIndexedId,
}

impl IdentityIndexData for SequenceIndexData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

/// Index identity by the sequence id
#[derive(Debug)]
pub struct SequenceIndex(TableEntity<SequenceIndexData>);

impl SequenceIndex {
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
            payload: SequenceIndexData {
                indexed_id: IdentityIndexedId {
                    identity_id: core.id.clone(),
                },
            },
        })
    }
}

impl IdentityIndex for SequenceIndex {
    type Index = SequenceIndexData;

    fn from_entity(entity: TableEntity<SequenceIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntity<SequenceIndexData> {
        self.0
    }

    fn into_data(self) -> SequenceIndexData {
        self.0.payload
    }

    fn data(&self) -> &SequenceIndexData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut SequenceIndexData {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.row_key
    }
}
