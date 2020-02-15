use super::{Identity, IdentityCategory, IdentityCore, IdentityData};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Data associated to a user identity
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserIdentityData {
    #[serde(flatten)]
    pub core: IdentityCore,
    pub password_hash: String,
}

impl IdentityData for UserIdentityData {
    fn core(&self) -> &IdentityCore {
        &self.core
    }
}

/// Identity assigned to a user
#[derive(Debug)]
pub struct UserIdentity(TableEntity<UserIdentityData>);

impl UserIdentity {
    pub fn new(
        id: String,
        sequence_id: u64,
        salt: String,
        name: String,
        email: Option<String>,
        password_hash: String,
    ) -> UserIdentity {
        let (partition_key, row_key) = Self::entity_keys(&id);
        UserIdentity(TableEntity {
            partition_key,
            row_key,
            etag: None,
            timestamp: None,
            payload: UserIdentityData {
                core: IdentityCore {
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

impl Identity for UserIdentity {
    type Data = UserIdentityData;

    fn entity_keys(id: &str) -> (String, String) {
        (format!("id-{}", &id[0..2]), id.to_string())
    }

    fn from_entity(entity: TableEntity<UserIdentityData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntity<UserIdentityData> {
        self.0
    }

    fn into_data(self) -> UserIdentityData {
        self.0.payload
    }

    fn data(&self) -> &UserIdentityData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut UserIdentityData {
        &mut self.0.payload
    }
}
