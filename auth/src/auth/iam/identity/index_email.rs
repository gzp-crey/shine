use super::{CoreIdentityIndexedData, EncodedEmail, Identity, IdentityCategory, IndexIdentityData, IndexIdentityEntity};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Storage type for index by email
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IndexEmailData {
    #[serde(flatten)]
    pub indexed_id: CoreIdentityIndexedData,
}

impl IndexIdentityData for IndexEmailData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

pub type IndexEmail = IndexIdentityEntity<IndexEmailData>;

impl IndexEmail {
    pub fn entity_keys(cat: IdentityCategory, email: &EncodedEmail) -> (String, String) {
        match cat {
            IdentityCategory::User => (format!("x_user_email-{}", email.prefix(2)), email.as_str().to_owned()),
        }
    }

    pub fn from_identity<T>(identity: &T) -> Option<Self>
    where
        T: Identity,
    {
        let core = identity.core();
        if let Some(ref email) = core.email {
            let (partition_key, row_key) = Self::entity_keys(core.category, email);
            Some(Self(TableEntity {
                partition_key,
                row_key,
                etag: None,
                timestamp: None,
                payload: IndexEmailData {
                    indexed_id: CoreIdentityIndexedData {
                        identity_id: core.id.clone(),
                    },
                },
            }))
        } else {
            None
        }
    }
}
