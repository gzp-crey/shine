use super::{CoreIdentityIndexedData, Identity, IndexIdentityData, IndexIdentityEntity, ValidatedName};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to an identity index by name
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IndexNameData {
    #[serde(flatten)]
    pub indexed_id: CoreIdentityIndexedData,
}

impl IndexIdentityData for IndexNameData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

/// Index identity by name
pub type IndexName = IndexIdentityEntity<IndexNameData>;

impl IndexName {
    pub fn entity_keys(name: &ValidatedName) -> (String, String) {
        (format!("x_name-{}", name.prefix(2)), name.as_str().to_owned())
    }

    pub fn from_identity<T>(identity: &T) -> Self
    where
        T: Identity,
    {
        let core = identity.core();
        let name = &core.name;
        let (partition_key, row_key) = Self::entity_keys(name);
        Self(TableEntity {
            partition_key,
            row_key,
            etag: None,
            timestamp: None,
            payload: IndexNameData {
                indexed_id: CoreIdentityIndexedData {
                    identity_id: core.id.clone(),
                },
            },
        })
    }
}
