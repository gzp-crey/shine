use azure_sdk_storage_table::TableEntity;
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
    remote: String,

    agent: String,

    #[serde(with = "datetime")]
    issued: DateTime<Utc>,

    refresh_count: u64,

    #[serde(with = "datetime")]
    refreshed: DateTime<Utc>,

    #[serde(with = "opt_datetime")]
    disabled: Option<DateTime<Utc>>,
}

impl SessionData {
    pub fn remote(&self) -> &str {
        &self.remote
    }

    pub fn agent(&self) -> &str {
        &self.agent
    }

    pub fn issue_date(&self) -> DateTime<Utc> {
        self.issued
    }

    pub fn refresh_count(&self) -> u64 {
        self.refresh_count
    }

    pub fn refresh_date(&self) -> DateTime<Utc> {
        self.refreshed
    }

    pub fn disable_date(&self) -> Option<DateTime<Utc>> {
        self.disabled
    }
}

/// The session of a user. Only users may have a session, other type of identites cannot log in and thus cannot
/// have session.
#[derive(Debug)]
pub struct Session(TableEntity<SessionData>);

impl Session {
    pub fn entity_keys(id: &str, key: &str) -> (String, String) {
        (format!("id-{}", id), key.to_owned())
    }

    pub fn new(id: String, key: String, site: &SiteInfo) -> Session {
        let (partition_key, row_key) = Self::entity_keys(&id, &key);

        Session(TableEntity {
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

    pub fn from_entity(entity: TableEntity<SessionData>) -> Self {
        Self(entity)
    }

    pub fn into_entity(self) -> TableEntity<SessionData> {
        self.0
    }

    pub fn data(&self) -> &SessionData {
        &self.0.payload
    }

    pub fn id(&self) -> &str {
        &self.0.partition_key.splitn(2, '-').skip(1).next().unwrap()
    }

    pub fn key(&self) -> &str {
        &self.0.row_key
    }

    pub fn invalidate(&mut self) {
        let data = &mut self.0.payload;
        data.disabled = Some(Utc::now());
    }

    pub fn is_invalidated(&self) -> bool {
        let data = &self.0.payload;
        data.disabled.is_some()
    }

    pub fn refresh(&mut self) {
        let data = &mut self.0.payload;
        data.refresh_count += 1;
        data.refreshed = Utc::now();
    }
}

impl From<Session> for SessionKey {
    fn from(session: Session) -> SessionKey {
        SessionKey::new(session.key().to_string())
    }
}
