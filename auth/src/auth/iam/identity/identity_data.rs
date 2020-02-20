use super::{EncodedEmail, EncodedName};
use azure_sdk_storage_table::TableEntity;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use std::str::Utf8Error;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum IdentityCategory {
    User,
    //Application,
    //Google,
    //Facebook,
}

/// Common data associated to each identity
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CoreIdentityData {
    pub id: String,
    pub sequence_id: u64,
    pub salt: String,
    pub category: IdentityCategory,
    pub name: EncodedName,
    pub email: Option<EncodedEmail>,
    pub email_validated: bool,
}

/// Identity data
pub trait IdentityData: Serialize + DeserializeOwned {
    /// Return the identity core, the common properties for all type of identites
    fn core(&self) -> &CoreIdentityData;
}

impl IdentityData for CoreIdentityData {
    fn core(&self) -> &CoreIdentityData {
        &self
    }
}

/// Identity
pub trait Identity {
    type Data: IdentityData;

    /// Generate partition and row keys from the id of an identity
    fn entity_keys(id: &str) -> (String, String);

    /// Create Self from the stored table entity
    fn from_entity(data: TableEntity<Self::Data>) -> Self
    where
        Self: Sized;

    /// Create a the table entity to store from Self
    fn into_entity(self) -> TableEntity<Self::Data>;

    /// Return the associated data
    fn into_data(self) -> Self::Data;

    /// Return the data associated to an identity
    fn data(&self) -> &Self::Data;

    /// Return the mutable data associated to an identity
    fn data_mut(&mut self) -> &mut Self::Data;

    /// Return the identity core, the common properties for all type of identites
    fn core(&self) -> &CoreIdentityData {
        self.data().core()
    }
}

#[derive(Debug)]
pub struct IdentityEntity<D: IdentityData>(TableEntity<D>);

impl<D> Identity for IdentityEntity<D>
where
    D: IdentityData,
{
    fn entity_keys(id: &str) -> (String, String) {
        (format!("id-{}", &id[0..2]), id.to_string())
    }

    fn from_entity(entity: TableEntity<D>) -> Self {
        Self(entity)
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
}

/// Identity when no category dependent data is required
pub type CoreIdentity = IdentityEntity<CoreIdentityData>;
