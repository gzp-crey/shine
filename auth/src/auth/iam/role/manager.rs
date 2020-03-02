use crate::auth::iam::{IAMConfig, IAMError};
use gremlin_client::{aio::GremlinClient, ConnectionOptions, GraphSON, GremlinError};
use serde::Serialize;
use shine_core::gremlin_utils::{query_value, query_vec};

/// A vector of a roles
pub type Roles = Vec<String>;

/// Role with inheritance information
#[derive(Debug, Serialize)]
pub struct InheritedRole {
    pub role: String,
    pub inherited_from: Option<String>,
}

/// A vector of roles with inheritance information
pub type InheritedRoles = Vec<InheritedRole>;

/// Manage the role database
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
            .ssl(true)
            .credentials(&config.graph_db_user, &config.graph_db_password)
            .serializer(GraphSON::V2)
            .build();
        let db = GremlinClient::connect(connection).await?;

        Ok(RoleManager { db: db })
    }

    pub async fn create_role(&self, role: &str) -> Result<(), IAMError> {
        let response = query_value::<String>(
            &self.db,
            r#"g.v().has('role','name',role).fold()
                .coalesce(
                    // if role already present return 'conflict'
                    unfold().constant('conflict'),

                    // create new role, return 'done'
                    addV('role').property('name',role).constant('done')
                )
            "#,
            &[("role", &role)],
        )
        .await?;

        match response.as_str() {
            "conflict" => Err(IAMError::RoleTaken),
            "done" => Ok(()),
            r => Err(GremlinError::Generic(format!("Unexpected query response: {}", r)).into()),
        }
    }

    pub async fn get_roles(&self) -> Result<Roles, IAMError> {
        Ok(query_vec::<String>(
            &self.db,
            r#"
                g.v().hasLabel('role').values('name');
            "#,
            &[],
        )
        .await?)
    }

    pub async fn delete_role(&self, role: &str) -> Result<(), IAMError> {
        let response = query_value::<String>(
            &self.db,
            r#"
                g.V().has('role','name', role)
                    .sideEffect(drop()).fold()
                    .coalesce(
                        unfold().constant('done'),
                        constant('missing')
                    )
            "#,
            &[("role", &role)],
        )
        .await?;

        match response.as_str() {
            "missing" => Err(IAMError::RoleNotFound),
            "done" => Ok(()),
            r => Err(GremlinError::Generic(format!("Unexpected query response: {}", r)).into()),
        }
    }

    pub async fn inherit_role(&self, role: &str, inherited_role: &str) -> Result<(), IAMError> {
        let response = query_vec::<String>(
            &self.db,
            r#"
                g.v().has('role','name',inherited_role)
                .coalesce(
                    // if the new edge creates a cycle, return the path 
                    __.repeat(out('has_role').dedup()).until(has('role','name',role))
                        .path().by('name').limit(1).unfold(),
                        
                    // if edge is already present, return 'conflict'
                    __.in('has_role').has('role','name',role).constant('conflict'),

                    // create the new edge, return 'done' 
                    __.addE('has_role').from(v().has('role','name',role)).constant('done')
                )
            "#,
            &[("role", &role), ("inherited_role", &inherited_role)],
        )
        .await?;

        match response.len() {
            0 => Err(IAMError::RoleNotFound),
            1 => {
                let response = response.first().unwrap();
                match response.as_str() {
                    "conflict" => Err(IAMError::HasRoleTaken),
                    "done" => Ok(()),
                    r => Err(GremlinError::Generic(format!("Unexpected query response: {}", r)).into()),
                }
            }
            _ => Err(IAMError::HasRoleCycle(response)),
        }
    }

    pub async fn disherit_role(&self, role: &str, inherited_role: &str) -> Result<(), IAMError> {
        let _ = self
            .db
            .execute(
                r#"g.V().has('role','name',role)
                    .out('has_role').has('role','name',inherited_role)
                    .drop()"#,
                &[("role", &role), ("inherited_role", &inherited_role)],
            )
            .await?;
        Ok(())
    }

    pub async fn create_identity(&self, identity: &str) -> Result<(), IAMError> {
        let response = query_value::<String>(
            &self.db,
            r#"g.v().has('identity','name',identity).fold()
                .coalesce(
                    // if identity already present return 'conflict'
                    unfold().constant('conflict'),

                    // create new identity, return 'done'
                    addV('identity').property('name',identity).constant('done')
                )
            "#,
            &[("identity", &identity)],
        )
        .await?;

        match response.as_str() {
            "conflict" => Err(IAMError::Internal(format!("Identity {} already registered", identity))),
            "done" => Ok(()),
            r => Err(GremlinError::Generic(format!("Unexpected query response: {}", r)).into()),
        }
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
}
