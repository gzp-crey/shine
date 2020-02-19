use super::{EncodedEmail, Identity, IdentityCategory, IdentityIndex, IdentityIndexData, IdentityIndexedId};
use azure_sdk_storage_table::TableEntity;
use serde::{Deserialize, Serialize};

/// Storage type for index by email
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmailIndexData {
    #[serde(flatten)]
    pub indexed_id: IdentityIndexedId,
}

impl IdentityIndexData for EmailIndexData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

#[derive(Debug)]
pub struct EmailIndex(TableEntity<EmailIndexData>);

impl EmailIndex {
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
            Some(EmailIndex(TableEntity {
                partition_key,
                row_key,
                etag: None,
                timestamp: None,
                payload: EmailIndexData {
                    indexed_id: IdentityIndexedId {
                        identity_id: core.id.clone(),
                    },
                },
            }))
        } else {
            None
        }
    }
}

impl IdentityIndex for EmailIndex {
    type Index = EmailIndexData;

    fn from_entity(entity: TableEntity<EmailIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntity<EmailIndexData> {
        self.0
    }

    fn into_data(self) -> EmailIndexData {
        self.0.payload
    }

    fn data(&self) -> &EmailIndexData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut EmailIndexData {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.row_key
    }
}
