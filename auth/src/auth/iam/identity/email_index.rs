use super::{Identity, IdentityIndex, IdentityIndexData, IdentityIndexedId};
use azure_sdk_storage_table::TableEntry;
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
pub struct EmailIndex(TableEntry<EmailIndexData>);

impl EmailIndex {
    pub fn entity_keys(email: &str) -> (String, String) {
        (format!("x_email-{}", &email[0..2]), email.to_string())
    }

    pub fn from_identity<T>(identity: &T) -> Option<Self>
    where
        T: Identity,
    {
        let core = identity.core();
        if let Some(ref email) = core.email {
            let (partition_key, row_key) = Self::entity_keys(email);
            Some(EmailIndex(TableEntry {
                partition_key,
                row_key,
                etag: None,
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

    fn from_entity(entity: TableEntry<EmailIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntry<EmailIndexData> {
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
