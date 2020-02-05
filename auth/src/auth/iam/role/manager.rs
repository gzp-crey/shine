use super::RoleMap;
use crate::auth::iam::{IAMConfig, IAMError};
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::table::{TableService, TableStorage};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct RoleManager {
    db: TableStorage,
}

// Handling identites
impl RoleManager {
    pub async fn new(config: &IAMConfig) -> Result<Self, IAMError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let db = TableStorage::new(table_service.clone(), "roles");

        db.create_if_not_exists().await?;

        Ok(RoleManager { db: db })
    }

    /// Return all the available roles
    pub async fn get_roles(&self) -> Result<RoleMap, IAMError> {
        ///todo: read from db
        let mut roles = HashMap::new();
        roles.insert("reader".to_string(), vec![]);
        roles.insert("writer".to_string(), vec![]);
        roles.insert("support".to_string(), vec!["reader".to_string()]);
        roles.insert(
            "admin".to_string(),
            vec!["reader".to_string(), "writer".to_string(), "support".to_string()],
        );

        Ok(roles)
    }

    /// Find all the inherited roles recursively for the given roles.
    pub fn resolve_roles(&self, mut roles: Vec<String>, role_map: &RoleMap) -> Vec<String> {
        let mut result = HashSet::new();
        while let Some(role) = roles.pop() {
            if result.contains(&role) {
                continue;
            }
            if let Some(derived_role) = role_map.get(&role) {
                roles.extend(derived_role.iter().map(|r| r.to_owned()));
            }
            result.insert(role);
        }
        result.drain().collect()
    }

    pub async fn get_roles_by_identity(&self, id: &str, include_derived: bool) -> Result<Vec<String>, IAMError> {
        //todo read from db
        let roles = vec!["test1".to_owned()];

        if include_derived {
            let role_map = self.get_roles().await?;
            Ok(self.resolve_roles(roles, &role_map))
        } else {
            Ok(roles)
        }
    }
}
