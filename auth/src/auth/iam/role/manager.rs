use crate::auth::iam::{IAMConfig, IAMError};
use gremlin_client::{ConnectionOptions, GremlinClient};

pub type Role = String;
pub type Roles = Vec<String>;

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

    /// Return all the available roles
    pub async fn get_roles(&self) -> Result<Roles, IAMError> {
        let results = self.db.execute("g.V(1).properties()", &[]);
        println!("result: {:?}", results);
        Err(IAMError::Internal("not implemented".to_owned()))
    }

    pub async fn create_role(&self, role: &str, derive: Vec<String>) -> Result<Role, IAMError> {
        unimplemented!()
    }

    pub async fn get_roles_by_identity(&self, id: &str, include_derived: bool) -> Result<Roles, IAMError> {
        unimplemented!()
    }
}
