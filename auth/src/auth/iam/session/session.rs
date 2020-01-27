use azure_sdk_storage_table::TableEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shine_core::{
    azure_utils::serde::{datetime, opt_datetime},
    session::SessionKey,
    siteinfo::SiteInfo,
};

/// Data associated to a session
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionData {
    pub remote: String,

    pub agent: String,

    #[serde(with = "datetime")]
    pub issued: DateTime<Utc>,

    pub refresh_count: u64,

    #[serde(with = "datetime")]
    pub refreshed: DateTime<Utc>,

    #[serde(with = "opt_datetime")]
    pub disabled: Option<DateTime<Utc>>,
}

/// The session of a user. Only users may have a session, other type of identites cannot log in and thus cannot
/// have session.
#[derive(Debug)]
pub struct Session(TableEntry<SessionData>);

impl Session {
    pub fn entity_keys(user_id: &str, key: &str) -> (String, String) {
        (user_id.to_owned(), key.to_owned())
    }

    pub fn new(user_id: String, key: String, site: &SiteInfo) -> Session {
        let (partition_key, row_key) = Self::entity_keys(&user_id, &key);

        Session(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: SessionData {
                remote: site.remote().to_string(),
                agent: site.agent().to_string(),
                issued: Utc::now(),
                refresh_count: 0,
                refreshed: Utc::now(),
                disabled: None,
            },
        })
    }

    pub fn from_entity(entity: TableEntry<SessionData>) -> Self {
        Self(entity)
    }

    pub fn into_entity(self) -> TableEntry<SessionData> {
        self.0
    }

    pub fn data(&self) -> &SessionData {
        &self.0.payload
    }

    pub fn data_mut(&mut self) -> &mut SessionData {
        &mut self.0.payload
    }

    pub fn id(&self) -> &str {
        &self.0.partition_key
    }

    pub fn key(&self) -> &str {
        &self.0.row_key.splitn(2, '-').skip(1).next().unwrap()
    }
}

impl From<Session> for SessionKey {
    fn from(session: Session) -> SessionKey {
        SessionKey::new(session.key().to_string())
    }
}
