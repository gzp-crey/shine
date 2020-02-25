use crate::auth::iam::{IAMConfig, IAMError};
use futures::stream::StreamExt;
use gremlin_client::{aio::GremlinClient, ConnectionOptions, GraphSON};

/// Basic type of a role
pub type Role = String;

/// A vector of a roles
pub type Roles = Vec<String>;

/// Role with inheritance information
pub struct InheritedRole {
    role: String,
    inherited_from: Option<String>,
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
        let mut result_stream = self
            .db
            .execute(
                r#"g.v().has('role','name',role).fold()
                    .coalesce(
                        // if role already present return 'conflict'
                        unfold().map(constant('conflict')),

                        // create new role, return 'done'
                        addV('role').property('name',role)
                            .map(constant('done'))
                    )
                "#,
                &[("role", &role)],
            )
            .await?;

        let result = result_stream
            .next()
            .await
            .ok_or(IAMError::Internal("Missing query result".to_owned()))??;

        match result.take::<String>()?.as_str() {
            "conflict" => Err(IAMError::RoleTaken),
            "done" => Ok(()),
            r => Err(IAMError::Internal(format!("Unexpected query response: {}", r))),
        }
    }

    pub async fn get_roles(&self) -> Result<Roles, IAMError> {
        let result = self
            .db
            .execute(r#"g.v().hasLabel('role').values('name');"#, &[])
            .await?
            .map(|e| e.unwrap().take::<String>().unwrap())
            .collect()
            .await;
        Ok(result)
    }

    pub async fn delete_role(&self, role: &str) -> Result<(), IAMError> {
        let _ = self
            .db
            .execute(r#"g.V().has('role','name',role).drop();"#, &[("role", &role)])
            .await?;
        Ok(())
    }

    pub async fn inherit_role(&self, role: &str, inherited_role: &str) -> Result<(), IAMError> {
        let result: Vec<String> = self
            .db
            .execute(
                r#"
                    g.v().has('role','name',inherited_role)
                    .coalesce(
                        // if the new edge creates a cycle, return the path 
                        __.repeat(out('has_role').dedup()).until(has('role','name',role))
                            .path().by('name').limit(1).unfold(),                                           
                            
                        // if edge is already present, return 'conflict'
                        __.in('has_role').has('role','name',role)
                            .map(constant('conflict')),       

                        // create the new edge, return 'ok' 
                        __.addE('has_role').from(v().has('role','name',role))
                            .map(constant('ok'))  
                    )
                "#,
                &[("role", &role), ("inherited_role", &inherited_role)],
            )
            .await?
            .map(|e| e.unwrap().take::<String>().unwrap())
            .collect()
            .await;
        println!("res: {:?}", result);
        /*match result.as_slice() {
            &["conflict"] => IAMError::Role
        }*/
        Ok(())
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
