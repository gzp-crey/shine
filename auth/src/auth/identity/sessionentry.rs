use super::identityentry::IdentityIndex;
use azure_sdk_storage_table::TableEntry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shine_core::{serde::date_serializer, session::SessionKey, siteinfo::SiteInfo};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Session {
    pub remote: String,
    pub agent: String,

    #[serde(with = "date_serializer")]
    pub issued: Utc,
}

#[derive(Debug)]
pub struct SessionEntry(TableEntry<Session>);

impl SessionEntry {
    pub fn entity_keys(user_id: &str, key: &str) -> String {
        (user_id.to_owned(), format!("session-{}", key))
    }

    pub fn new(user_id: String, key: String, site: &SiteInfo) -> SessionEntry {
        let (partition_key, row_key) = Self::entity_keys(&user_id, &key);

        SessionEntry(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: Session {
                remote: site.remote().to_string(),
                agent: site.agent().to_string(),
                issued: Utc::now(),
            },
        })
    }

    pub fn from_entry(entry: TableEntry<Session>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<Session> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<Session> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.partition_key
    }

    pub fn key(&self) -> &str {
        &self.0.row_key.split("-").first().unwrap()
    }

    pub fn data(&self) -> &Session {
        &self.0.payload
    }
}

impl From<SessionEntry> for SessionKey {
    fn from(session: SessionEntry) -> SessionKey {
        let session = session.into_entry();
        SessionKey::new(session.row_key)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionIndex {
    #[serde(flatten)]
    pub identity_id: IdentityIndex,
}

#[derive(Debug)]
pub struct SessionIndexEntry(TableEntry<SessionIndex>);

impl SessionIndexEntry {
    pub fn entity_keys(key: &str) -> (String, String) {
        (format!("session_{}", &key[0..2]), key.to_owned())
    }

    pub fn from_identity(entry: &SessionEntry) -> Self {
        let key = entry.key();
        let user_id = entry.user_id();

        let (partition_key, row_key) = Self::entity_keys(&user_id, &key);

        Self(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: SessionIndex {
                identity_id: IdentityIndex {
                    user_id: user_id.to_owned(),
                },
            },
        })
    }

    pub fn from_entry(entry: TableEntry<SessionIndex>) -> Self {
        Self(entry)
    }

    pub fn into_entry(self) -> TableEntry<SessionIndex> {
        self.0
    }

    pub fn entry(&self) -> &TableEntry<SessionIndex> {
        &self.0
    }

    pub fn user_id(&self) -> &str {
        &self.0.payload.identity_id.user_id
    }

    pub fn key(&self) -> &str {
        &self.0.row_key
    }
}
