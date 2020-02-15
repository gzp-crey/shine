use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to an session index by key
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionIndexData {
    pub identity_id: String,
}

/// Index session by key
#[derive(Debug)]
pub struct SessionIndex(TableEntity<SessionIndexData>);

impl SessionIndex {
    pub fn entity_keys(key: &str) -> (String, String) {
        (format!("x_key-{}", &key[0..2]), key.to_string())
    }

    pub fn new(key: &str, id: &str) -> Self {
        let (partition_key, row_key) = Self::entity_keys(key);
        Self(TableEntity {
            partition_key,
            row_key,
            etag: None,
            timestamp: None,
            payload: SessionIndexData {
                identity_id: id.to_owned(),
            },
        })
    }

    pub fn from_entity(entity: TableEntity<SessionIndexData>) -> Self {
        Self(entity)
    }

    pub fn into_entity(self) -> TableEntity<SessionIndexData> {
        self.0
    }

    pub fn id(&self) -> &str {
        &self.0.payload.identity_id
    }

    pub fn key(&self) -> &str {
        &self.0.row_key
    }
}
