use super::IdentitySession;
use actix_web::Error as ActixError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserId {
    user_id: String,
    name: String,
    //roles: Vec<String>,
}

impl UserId {
    pub fn new(user_id: String, name: String, roles: Vec<String>) -> Self {
        UserId {
            user_id,
            name, /*, roles*/
        }
    }

    pub fn user_id(&self) -> &String {
        &self.user_id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    /*pub fn roles(&self) -> &Vec<String> {
        &self.roles
    }*/
}

impl UserId {
    pub fn from_session(session: &IdentitySession) -> Result<Option<Self>, ActixError> {
        session.get::<UserId>("identity")
    }

    pub fn to_session(self, session: &IdentitySession) -> Result<(), ActixError> {
        session.set("identity", self)
    }
}
