use crate::auth::iam::{IAMConfig, IAMError};
use azure_sdk_storage_table::{CloudTable, TableClient, TableEntity};
use futures::stream::StreamExt;
use percent_encoding::{self, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The description of a role
#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub derive: Vec<String>,
    pub version: String,
}

impl Role {
    fn from_entity(entity: TableEntity<RoleData>) -> Role {
        Role {
            name: entity.row_key,
            derive: entity.payload.get_roles(),
            version: entity.etag.unwrap_or_default(),
        }
    }

    fn into_entity(self) -> TableEntity<RoleData> {
        let (p, r) = RoleData::entity_keys(&self.name);
        TableEntity {
            partition_key: p,
            row_key: r,
            etag: Some(self.version),
            timestamp: None,
            payload: RoleData {
                derive: self.derive.join(","),
            },
        }
    }
}

pub type RoleMap = HashMap<String, Role>;

pub type Roles = Vec<String>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RoleData {
    derive: String,
}

impl RoleData {
    fn entity_keys(role: &str) -> (String, String) {
        ("role".to_string(), role.to_owned())
    }

    fn get_roles(&self) -> Roles {
        self.derive.split(",").map(|r| r.trim().to_string()).collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct IdentityRoleData {
    roles: String,
}

impl IdentityRoleData {
    pub fn entity_keys(id: &str) -> (String, String) {
        (format!("id-{}", &id[0..2]), id.to_owned())
    }

    pub fn get_roles(&self) -> Roles {
        self.roles.split(",").map(|r| r.trim().to_string()).collect()
    }
}

#[derive(Clone)]
pub struct RoleManager {
    db: CloudTable,
}

// Handling identites
impl RoleManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let client = TableClient::new(&config.storage_account, &config.storage_account_key)?;
        let db = CloudTable::new(client.clone(), "roles");
        db.create_if_not_exists().await?;

        Ok(RoleManager { db: db })
    }

    /// Return all the available roles
    pub async fn get_roles(&self) -> Result<RoleMap, IAMError> {
        // query all the roles
        let query = "PartitionKey eq 'role'";
        let query = format!("$filter={}", utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC));

        let mut stream = Box::pin(self.db.stream_query::<RoleData>(Some(&query)));
        let mut roles = HashMap::new();
        while let Some(Ok(segment)) = stream.next().await {
            for entity in segment {
                log::info!("etity: {:?}", entity);
                let role = Role::from_entity(entity);
                roles.insert(role.name.clone(), role);
            }
        }

        Ok(roles)
    }

    pub async fn is_valid_role(&self, role: &str) -> Result<bool, IAMError> {
        let (p, r) = RoleData::entity_keys(role);
        let entity = self.db.get::<RoleData>(&p, &r, None).await?;
        Ok(entity.is_some())
    }

    pub async fn get_role(&self, role: &str) -> Result<Role, IAMError> {
        let (p, r) = RoleData::entity_keys(role);
        if let Some(entity) = self.db.get::<RoleData>(&p, &r, None).await? {
            Ok(Role::from_entity(entity))
        } else {
            Err(IAMError::RoleNotFound)
        }
    }

    pub async fn create_role(&self, role: &str, derive: Vec<String>) -> Result<Role, IAMError> {
        let role = Role {
            name: role.to_owned(),
            derive: derive,
            version: "".to_string(),
        };
        let entity = role.into_entity();
        let entity = self.db.insert_entity::<RoleData>(entity).await?;
        Ok(Role::from_entity(entity))
    }

    pub async fn update_role(&self, role: Role) -> Result<Role, IAMError> {
        if role.version == "" || role.version == "*" {
            // avoid invalid or "any" etag
            return Err(IAMError::BadRequest(format!("Invalid role version: {}", role.version)));
        }
        let entity = role.into_entity();
        let entity = self.db.update_entity(entity).await?;
        Ok(Role::from_entity(entity))
    }

    /// Find all the inherited roles recursively for the given roles.
    pub fn resolve_roles(&self, mut roles: Vec<String>, role_map: &RoleMap) -> Vec<String> {
        let mut result = HashSet::new();
        while let Some(role) = roles.pop() {
            if result.contains(&role) {
                continue;
            }
            if let Some(role) = role_map.get(&role) {
                roles.extend(role.derive.iter().map(|r| r.to_owned()));
            }
            result.insert(role);
        }
        result.drain().collect()
    }

    pub async fn get_roles_by_identity(&self, id: &str, include_derived: bool) -> Result<Roles, IAMError> {
        let (p, r) = IdentityRoleData::entity_keys(id);
        if let Some(entity) = self.db.get::<IdentityRoleData>(&p, &r, None).await? {
            let roles = entity.payload.get_roles();
            if include_derived {
                let role_map = self.get_roles().await?;
                Ok(self.resolve_roles(roles, &role_map))
            } else {
                Ok(roles)
            }
        } else {
            Ok(vec![])
        }
    }
}
