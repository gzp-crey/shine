use crate::{Config, CookingError};
use shine_game::assets::io;
use sqlx::SqlitePool;

impl From<sqlx::Error> for CookingError {
    fn from(err: sqlx::Error) -> CookingError {
        CookingError::Db(format!("{}", err))
    }
}

pub struct SourceEntry {
    pub local_url: String,
    pub config_hash: String,
    pub hash: String,
    pub cooked_id: String,
}

pub struct Dependency {
    pub parent_url: String,
    pub scope: String,
    pub child_url: String,
}

//Manage local sources to speed up compilation
#[derive(Clone)]
pub struct LocalDB {
    pool: SqlitePool,
    config_hash: String,
}

impl LocalDB {
    pub async fn new(config: &Config) -> Result<LocalDB, CookingError> {
        let pool = SqlitePool::new(&config.local_db_connection).await?;
        let config_hash = io::sha256_bytes(&bincode::serialize(config)?);
        Ok(LocalDB { pool, config_hash })
    }

    ///Remove all entries not matching the current config
    pub async fn purge_config(&self) -> Result<(), CookingError> {
        unimplemented!()
    }

    pub async fn get_info(&self, local_url: &str) -> Result<Option<SourceEntry>, CookingError> {
        unimplemented!()
    }

    pub async fn clear_info(&self, local_url: &str) -> Result<(), CookingError> {
        unimplemented!()
    }

    pub async fn set_info(&self, local_url: &str, hash: &str, cooked_id: &str) -> Result<(), CookingError> {
        let entry = SourceEntry {
            local_url: local_url.to_owned(),
            config_hash: self.config_hash.to_owned(),
            hash: hash.to_owned(),
            cooked_id: cooked_id.to_owned(),
        };
        unimplemented!()
    }

    /// Return the direct (local) dependencies of a resource
    pub async fn get_dependency(&self, local_url: &str) -> Result<Vec<Dependency>, CookingError> {
        unimplemented!()
    }

    /// Set the direct (local) dependencies of a resource
    pub async fn set_dependecy(&self, local_url: &str, deps: Vec<(String,String)>) -> Result<(), CookingError> {
        unimplemented!()
    }

    pub async fn cache_by_data(&self, local_url: &str, data: &[u8]) -> Result<CacheState<'_>, CookingError> {
        let hash = io::sha256_bytes(data);
        self.cache_by_hash(local_url, &hash).await
    }

    pub async fn cache_by_hash(&self, local_url: &str, hash: &str) -> Result<CacheState<'_>, CookingError> {
        if let Some(info) = self.get_info(local_url).await? {
            if info.config_hash != self.config_hash || hash != info.hash {
                log::debug!("[{}] Cache config or local content changed ...", local_url);
                Ok(CacheState::Incomplete(IncompleteCache::new(
                    self,
                    local_url.to_owned(),
                    hash.to_owned(),
                )))
            } else {
                log::debug!("[{}] Content is up to date ...", local_url);
                let dependencies = self.get_dependency(local_url).await?;
                log::trace!("[{}] Local dependencies: {:?}", local_url, dependencies);
                Ok(CacheState::Complete(CompleteCache::new(
                    self,
                    local_url.to_owned(),
                    info.cooked_id,
                    dependencies,
                )))
            }
        } else {
            Ok(CacheState::Incomplete(IncompleteCache::new(
                self,
                local_url.to_owned(),
                hash.to_owned(),
            )))
        }
    }
}

/// Caching context for cook.
pub enum CacheState<'a> {
    Incomplete(IncompleteCache<'a>),
    Complete(CompleteCache<'a>),
}

/// Caching context when cooking is required
pub struct IncompleteCache<'a> {
    db: &'a LocalDB,
    local_url: String,
    cooked_id: Option<String>,
    hash: String,
    dependencies: Vec<(String,String))>,
}

impl<'a> IncompleteCache<'a> {
    fn new(db: &LocalDB, local_url: String, hash: String) -> IncompleteCache<'_> {
        IncompleteCache {
            db,
            local_url,
            cooked_id: None,
            hash,
            dependencies: Vec::new(),
        }
    }

    pub fn add_dependency(&mut self, scope: &str, dependency: &str) {
        self.dependencies.push((scope.to_owned(), dependency(to_owned())))
    }

    pub fn set_cooked_id(&mut self, url: &str) {
        self.cooked_id = Some(url.to_owned())
    }
}

impl<'a> Drop for IncompleteCache<'a> {
    fn drop(&mut self) {
        // todo: write back to db
    }
}

/// Caching context when cooking is not required
pub struct CompleteCache<'a> {
    db: &'a LocalDB,
    local_url: String,
    cooked_id: String,
    dependencies: Vec<Dependency>,
}

impl<'a> CompleteCache<'a> {
    fn new(db: &LocalDB, local_url: String, cooked_id: String, dependencies: Vec<String>) -> CompleteCache<'_> {
        CompleteCache {
            db,
            local_url,
            cooked_id,
            dependencies,
        }
    }

    pub fn dependencies(&mut self) -> &[Dependency] {
        &self.dependencies
    }

    pub fn cooked_id(&mut self) -> &str {
        &self.cooked_id
    }
}
