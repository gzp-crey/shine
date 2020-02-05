mod manager;

pub use self::manager::*;
use std::collections::HashMap;

pub type Roles = Vec<String>;
pub type RoleMap = HashMap<String, Roles>;
