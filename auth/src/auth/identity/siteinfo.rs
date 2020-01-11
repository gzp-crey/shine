use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteInfo {
    pub ip: String,
    pub agent: String,
}
