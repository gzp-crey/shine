use super::identityentry::IdentityIndex;
use shine_core::siteinfo::SiteInfo;
use shine_core::session::SessionKey;
use azure_sdk_storage_table::TableEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Login {
    #[serde(flatten)]
    pub site: SiteInfo,
}

#[derive(Debug)]
pub struct LoginEntry(TableEntry<Login>);

impl LoginEntry {
    pub fn new(user_id: String, key: String, site: SiteInfo) -> LoginEntry {
        LoginEntry(TableEntry {
            partition_key: user_id,
            row_key: key,
            etag: None,
            payload: Login { site },
        })
    }

    pub fn from_entry(entry: TableEntry<Login>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<Login> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<Login> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.partition_key
    }

    pub fn key(&self) -> &str {
        &self.0.row_key
    }

    pub fn data(&self) -> &Login {
        &self.0.payload
    }
}

impl From<LoginEntry> for SessionKey {
    fn from(session: LoginEntry) -> SessionKey {
        let session = session.into_entry();
        SessionKey::new(session.row_key)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LoginIndex {
    #[serde(flatten)]
    pub identity_id: IdentityIndex,
}

#[derive(Debug)]
pub struct LoginIndexEntry(TableEntry<LoginIndex>);

impl LoginIndexEntry {
    pub fn generate_partion_key(key: &str) -> String {
        format!("login_{}", &key[0..2])
    }

    pub fn from_identity(entry: &LoginEntry) -> Self {
        let key = entry.key();
        let user_id = entry.user_id();

        Self(TableEntry {
            partition_key: Self::generate_partion_key(key),
            row_key: key.to_string(),
            etag: None,
            payload: LoginIndex {
                identity_id: IdentityIndex {
                    user_id: user_id.to_owned(),
                },
            },
        })
    }

    pub fn from_entry(entry: TableEntry<LoginIndex>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<LoginIndex> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<LoginIndex> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.payload.identity_id.user_id
    }

    pub fn key(&self) -> &str {
        &self.0.row_key
    }
}
