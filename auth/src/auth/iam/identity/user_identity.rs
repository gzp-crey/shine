use super::{CoreIdentityData, EncodedEmail, EncodedName, Identity, IdentityCategory, IdentityData, IdentityEntity};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to a user identity
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserIdentityData {
    #[serde(flatten)]
    pub core: CoreIdentityData,
    pub password_hash: String,
}

impl IdentityData for UserIdentityData {
    fn core(&self) -> &CoreIdentityData {
        &self.core
    }
}

/// Identity assigned to a user
pub type UserIdentity = IdentityEntity<UserIdentityData>;

impl UserIdentity {
    pub fn new(
        id: String,
        sequence_id: u64,
        salt: String,
        name: EncodedName,
        email: Option<EncodedEmail>,
        password_hash: String,
    ) -> Self {
        let (partition_key, row_key) = Self::entity_keys(&id);
        Self(TableEntity {
            partition_key,
            row_key,
            etag: None,
            timestamp: None,
            payload: UserIdentityData {
                core: CoreIdentityData {
                    id,
                    sequence_id,
                    salt,
                    name,
                    category: IdentityCategory::User,
                    email,
                    email_validated: false,
                },
                password_hash,
            },
        })
    }
}
