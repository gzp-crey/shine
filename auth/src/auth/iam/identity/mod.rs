use azure_sdk_storage_table::TableEntity;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use std::str::Utf8Error;

mod email_index;
mod manager;
mod name_index;
mod sequence_index;
mod user_identity;

pub use self::email_index::*;
pub use self::manager::*;
pub use self::name_index::*;
pub use self::sequence_index::*;
pub use self::user_identity::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedName(String);

impl EncodedName {
    pub fn from_raw<S: ToString>(raw: S) -> EncodedName {
        EncodedName(encode_safe_key(&raw.to_string()))
    }

    pub fn to_raw(&self) -> Result<String, Utf8Error> {
        decode_safe_key(&self.0)
    }

    pub fn prefix(&self, len: usize) -> &str {
        &self.0[..self.0.char_indices().nth(len).unwrap().0]
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedEmail(String);

impl EncodedEmail {
    pub fn from_raw<S: ToString>(raw: S) -> EncodedEmail {
        EncodedEmail(encode_safe_key(&raw.to_string()))
    }

    pub fn to_raw(&self) -> Result<String, Utf8Error> {
        decode_safe_key(&self.0)
    }

    pub fn prefix(&self, len: usize) -> &str {
        &self.0[..self.0.char_indices().nth(len).unwrap().0]
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
pub struct IdentityCore {
    pub id: String,
    pub sequence_id: u64,
    pub salt: String,
    pub category: IdentityCategory,
    pub name: EncodedName,
    pub email: Option<EncodedEmail>,
    pub email_validated: bool,
}

/// The index used to reference an identity during indexing
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityIndexedId {
    pub identity_id: String,
}

/// Identity data
pub trait IdentityData: Serialize + DeserializeOwned {
    /// Return the identity core, the common properties for all type of identites
    fn core(&self) -> &IdentityCore;
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
    fn core(&self) -> &IdentityCore {
        self.data().core()
    }
}

/// Data associated to each identity
pub trait IdentityIndexData: Serialize + DeserializeOwned {
    /// Id of the associated identity
    fn id(&self) -> &str;
}

pub trait IdentityIndex {
    type Index: IdentityIndexData;

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
