use crate::auth::iam::fingerprint::Fingerprint;
use azure_sdk_storage_table::TableEntity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shine_core::{kernel::identity::SessionKey, serde_with};

/// Data associated to a session
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionData {
    agent: String,
    remote_ip: String,
    remote_continent: String,
    remote_country: String,

    #[serde(with = "serde_with::datetime")]
    issued: DateTime<Utc>,

    refresh_count: u64,

    #[serde(with = "serde_with::datetime")]
    refreshed: DateTime<Utc>,

    #[serde(with = "serde_with::opt_datetime")]
    disabled: Option<DateTime<Utc>>,
}

impl SessionData {
    pub fn check(&self, fingerprint: &Fingerprint) -> bool {
        // check agent
        if self.agent != fingerprint.agent() {
            log::debug!("fingerprint: agent has changed: {} -> {}", self.agent, fingerprint.agent());
            return false;
        }

        // check ip
        let ip = fingerprint.remote().map(|ip| ip.to_string()).unwrap_or("unknown".to_string());
        let country = fingerprint
            .location()
            .map(|l| l.country.to_owned())
            .unwrap_or("unknown".to_string());

        if self.remote_ip != ip {
            log::debug!("fingerprint: ip has changed: {} -> {}", self.remote_ip, ip);
            return false;
        }

        // check location
        if self.remote_country != country {
            log::debug!("fingerprint: country has changed: {} -> {}", self.remote_country, country);
            return false;
        }

        true
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

    pub fn new(id: String, key: String, fingerprint: &Fingerprint) -> Session {
        let (partition_key, row_key) = Self::entity_keys(&id, &key);

        Session(TableEntity {
            partition_key,
            row_key,
            etag: None,
            timestamp: None,
            payload: SessionData {
                agent: fingerprint.agent().to_string(),
                remote_ip: fingerprint.remote().map(|ip| ip.to_string()).unwrap_or("unknown".to_string()),
                remote_continent: fingerprint
                    .location()
                    .map(|l| l.continent.to_owned())
                    .unwrap_or("unknown".to_string()),
                remote_country: fingerprint
                    .location()
                    .map(|l| l.country.to_owned())
                    .unwrap_or("unknown".to_string()),
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

    pub fn check(&mut self, fingerprint: &Fingerprint, minimum_refresh_date: DateTime<Utc>) -> bool {
        let data = &self.0.payload;
        data.disabled.is_none() && data.refreshed >= minimum_refresh_date && data.check(fingerprint)
    }

    pub fn is_disabled(&self) -> bool {
        let data = &self.0.payload;
        data.disabled.is_some()
    }

    pub fn disable(&mut self) {
        let data = &mut self.0.payload;
        if data.disabled.is_none() {
            data.disabled = Some(Utc::now());
        }
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
