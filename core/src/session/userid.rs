use super::IdentitySession;
use crate::serde_with;
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::FromIterator;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserId {
    user_id: String,
    name: String,
    #[serde(with = "serde_with::hashset_list")]
    roles: HashSet<String>,
}

impl UserId {
    pub fn new(user_id: String, name: String, roles: Vec<String>) -> Self {
        UserId {
            user_id,
            name,
            roles: HashSet::from_iter(roles.into_iter()),
        }
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn roles(&self) -> &HashSet<String> {
        &self.roles
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }
}

impl UserId {
    pub fn from_session(session: &IdentitySession) -> Result<Option<Self>, ActixError> {
        session.get::<UserId>("id")
    }

    pub fn to_session(self, session: &IdentitySession) -> Result<(), ActixError> {
        session.set("id", self)
    }
}
