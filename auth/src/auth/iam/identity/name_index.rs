use super::{Identity, IdentityIndex, IdentityIndexData, IdentityIndexedId};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to an identity index by name
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NameIndexData {
    #[serde(flatten)]
    pub indexed_id: IdentityIndexedId,
}

impl IdentityIndexData for NameIndexData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

/// Index identity by name
#[derive(Debug)]
pub struct NameIndex(TableEntity<NameIndexData>);

impl NameIndex {
    pub fn entity_keys(name: &str) -> (String, String) {
        (format!("x_name-{}", &name[0..2]), name.to_string())
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
            payload: NameIndexData {
                indexed_id: IdentityIndexedId {
                    identity_id: core.id.clone(),
                },
            },
        })
    }
}

impl IdentityIndex for NameIndex {
    type Index = NameIndexData;

    fn from_entity(entity: TableEntity<NameIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntity<NameIndexData> {
        self.0
    }

    fn into_data(self) -> NameIndexData {
        self.0.payload
    }

    fn data(&self) -> &NameIndexData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut NameIndexData {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.row_key
    }
}
