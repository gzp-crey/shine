use crate::auth::iam::{IAMConfig, IAMError};
use gremlin_client::{ConnectionOptions, GremlinClient};

pub type Role = String;
pub type Roles = Vec<String>;

/// Role with inheritance information
pub struct InheritedRole {
    role: String,
    inherited_from: Option<String>,
}
pub type InheritedRoles = Vec<InheritedRole>;

#[derive(Clone)]
pub struct RoleManager {
    db: GremlinClient,
}

// Handling identites
impl RoleManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let connection = ConnectionOptions::builder()
            .host(&config.graph_db_host)
            .port(config.graph_db_port)
            .build();
        let db = GremlinClient::connect(connection)?;

        Ok(RoleManager { db: db })
    }

    pub async fn create_role(&self, _role: &str) -> Result<(), IAMError> {
        unimplemented!()
    }

    pub async fn get_roles(&self) -> Result<Roles, IAMError> {
        let results = self.db.execute("g.V(1).properties()", &[]);
        println!("result: {:?}", results);
        Err(IAMError::Internal("not implemented".to_owned()))
    }

    pub async fn inherit_role(&self, _role: &str, _inherited_role: &str) -> Result<(), IAMError> {
        unimplemented!()
    }

    pub async fn disherit_role(&self, _role: &str, _inherited_role: &str) -> Result<(), IAMError> {
        unimplemented!()
    }

    pub async fn add_identity_role(&self, _identity_id: &str, _role: &str) -> Result<InheritedRoles, IAMError> {
        unimplemented!()
    }

    pub async fn get_identity_roles(&self, _identity_id: &str, _include_inherited: bool) -> Result<InheritedRoles, IAMError> {
        unimplemented!()
    }

    pub async fn remove_identity_role(&self, _identity_id: &str, _role: &str) -> Result<InheritedRoles, IAMError> {
        unimplemented!()
    }

    pub async fn get_roles_by_identity(&self, _identity_id: &str, _include_derived: bool) -> Result<Roles, IAMError> {
        unimplemented!()
    }
}
