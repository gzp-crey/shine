use crate::session::UserId;
use azure_sdk_storage_table::TableEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityData {
    pub name: String,
    pub email: Option<String>,
    pub email_validate: bool,
    pub password_hash: String,
    //pub roles: Vec<String>,
}

pub struct IdentityEntry(TableEntry<IdentityData>);

impl IdentityEntry {
    pub fn new(id: String, name: String, email: Option<String>, password_hash: String) -> IdentityEntry {
        IdentityEntry(TableEntry {
            partition_key: id.clone(),
            row_key: id.clone(),
            etag: None,
            payload: IdentityData {
                name,
                email,
                email_validate: false,
                password_hash,
                //roles: vec![],
            },
        })
    }

    pub fn from_entry(entry: TableEntry<IdentityData>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<IdentityData> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<IdentityData> {
        &self.0
    }

    pub fn id(&self) -> &str {
        &self.0.row_key
    }

    pub fn identity(&self) -> &IdentityData {
        &self.0.payload
    }
}

impl From<IdentityEntry> for UserId {
    fn from(user: IdentityEntry) -> Self {
        let user = user.into_entry();
        UserId::new(user.row_key, user.payload.name, vec![] /*user.roles*/)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NameIndex {
    pub user_id: String,
}

pub struct NameIndexEntry(TableEntry<NameIndex>);

impl NameIndexEntry {
    pub fn from_identity(entry: &IdentityEntry) -> Self {
        Self(TableEntry {
            partition_key: entry.identity().name.clone(),
            row_key: entry.identity().name.clone(),
            etag: None,
            payload: NameIndex {
                user_id: entry.id().to_owned(),
            },
        })
    }

    pub fn id(&self) -> &str {
        &self.0.payload.user_id
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmailIndex {
    pub user_id: String,
}

pub struct EmailIndexEntry(TableEntry<EmailIndex>);

impl EmailIndexEntry {
    pub fn from_identity(entry: &IdentityEntry) -> Self {
        Self(TableEntry {
            partition_key: entry.identity().name.clone(),
            row_key: entry.identity().name.clone(),
            etag: None,
            payload: EmailIndex {
                user_id: entry.id().to_owned(),
            },
        })
    }

    pub fn id(&self) -> &str {
        &self.0.payload.user_id
    }
}
