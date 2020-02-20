use azure_sdk_storage_table::TableEntity;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use std::str::Utf8Error;

/// Data associated to each identity
pub trait IndexIdentityData: Serialize + DeserializeOwned {
    /// Id of the associated identity
    fn id(&self) -> &str;
}

/// The index used to reference an identity during indexing
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CoreIdentityIndexedData {
    pub identity_id: String,
}

impl IndexIdentityData for CoreIdentityIndexedData {
    fn id(&self) -> &str {
        &self.identity_id
    }
}

/// Indexing of an identity
pub trait IndexIdentity {
    type Index: IndexIdentityData;

    /// Create Self from the stored table entity
    fn from_entity(data: TableEntity<Self::Index>) -> Self
    where
        Self: Sized;

    /// Create a the table entity to store from Self
    fn into_entity(self) -> TableEntity<Self::Index>;

    /// Return the associated data
    fn into_data(self) -> Self::Index;

    /// Return the data associated to the index (and not to the identity)
    fn data(&self) -> &Self::Index;

    /// Return the mutable data associated to the index (and not to the identity)
    fn data_mut(&mut self) -> &mut Self::Index;

    /// The unique key to index
    fn index_key(&self) -> &str;

    /// Return the (unique) id of the identity
    fn id(&self) -> &str {
        self.data().id()
    }
}

#[derive(Debug)]
pub struct IndexIdentityEntity<D: IndexIdentityData>(TableEntity<D>);

impl<D> IndexIdentity for IndexIdentityEntity<D>
where
    D: IndexIdentityData,
{
    type Index = D;

    fn from_entity(data: TableEntity<D>) -> Self {
        Self(data)
    }

    fn into_entity(self) -> TableEntity<D> {
        self.0
    }

    fn into_data(self) -> D {
        self.0.payload
    }

    fn data(&self) -> &D {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut D {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.partition_key
    }
}

pub type CoreIdentityIndexed = IndexIdentityEntity<CoreIdentityIndexedData>;
