use shine_core::session::UserId;
use azure_sdk_storage_table::TableEntry;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EmptyEntry {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Identity {
    pub sequence_id: String,
    pub salt: String,
    pub name: String,
    pub email: Option<String>,
    pub email_validated: bool,
    pub password_hash: String,
    //pub roles: Vec<String>,
}

#[derive(Debug)]
pub struct IdentityEntry(TableEntry<Identity>);

impl IdentityEntry {
    pub fn generate_partion_key(id: &str) -> String {
        id[0..2].to_string()
    }

    pub fn new(
        id: String,
        sequence_id: String,
        salt: String,
        name: String,
        email: Option<String>,
        password_hash: String,
    ) -> IdentityEntry {
        IdentityEntry(TableEntry {
            partition_key: Self::generate_partion_key(&id),
            row_key: id,
            etag: None,
            payload: Identity {
                sequence_id,
                salt,
                name,
                email,
                email_validated: false,
                password_hash,
                //roles: vec![],
            },
        })
    }

    pub fn from_entry(entry: TableEntry<Identity>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<Identity> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<Identity> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.row_key
    }

    pub fn data(&self) -> &Identity {
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
pub struct IdentityIndex {
    pub user_id: String,
}

#[derive(Debug)]
pub struct IdentityIndexEntry(TableEntry<IdentityIndex>);

impl IdentityIndexEntry {
    pub fn from_entry(entry: TableEntry<IdentityIndex>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<IdentityIndex> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<IdentityIndex> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.payload.user_id
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NameIndex {
    #[serde(flatten)]
    pub identity_id: IdentityIndex,
}

#[derive(Debug)]
pub struct NameIndexEntry(TableEntry<NameIndex>);

impl NameIndexEntry {
    pub fn generate_partion_key(name: &str) -> String {
        format!("name_{}", &name[0..2])
    }

    pub fn from_identity(entry: &IdentityEntry) -> Self {
        let name = &entry.data().name;
        Self(TableEntry {
            partition_key: Self::generate_partion_key(name),
            row_key: name.to_string(),
            etag: None,
            payload: NameIndex {
                identity_id: IdentityIndex {
                    user_id: entry.user_id().to_owned(),
                },
            },
        })
    }

    pub fn from_entry(entry: TableEntry<NameIndex>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<NameIndex> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<NameIndex> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.payload.identity_id.user_id
    }

    pub fn name(&self) -> &str {
        &self.0.row_key
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmailIndex {
    #[serde(flatten)]
    pub identity_id: IdentityIndex,
}

#[derive(Debug)]
pub struct EmailIndexEntry(TableEntry<EmailIndex>);

impl EmailIndexEntry {
    pub fn generate_partion_key(email: &str) -> String {
        format!("email_{}", &email[0..2])
    }

    pub fn from_identity(entry: &IdentityEntry) -> Option<Self> {
        if let Some(ref email) = entry.data().email {
            Some(Self(TableEntry {
                partition_key: Self::generate_partion_key(email),
                row_key: email.clone(),
                etag: None,
                payload: EmailIndex {
                    identity_id: IdentityIndex {
                        user_id: entry.user_id().to_owned(),
                    },
                },
            }))
        } else {
            None
        }
    }

    pub fn from_entry(entry: TableEntry<EmailIndex>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<EmailIndex> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<EmailIndex> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.payload.identity_id.user_id
    }

    pub fn email(&self) -> &str {
        &self.0.row_key
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LoginKeyIndex {
    #[serde(flatten)]
    pub identity_id: IdentityIndex,
}
