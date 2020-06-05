use crate::{Config, CookingError, Dependency, TargetDB};
use shine_game::assets::{io, Url};
use sqlx::{
    self,
    sqlite::{SqlitePool, SqliteQueryAs},
};

#[derive(Debug, sqlx::FromRow)]
pub struct SourceCacheEntry {
    pub source_id: String,
    pub source_hash: String,
    pub cooked_url: String,
}

/// Manage sources to speed up compilation
#[derive(Clone)]
pub struct CacheDB {
    pool: SqlitePool,
    config_hash: String,
}

impl CacheDB {
    pub async fn new(config: &Config) -> Result<CacheDB, CookingError> {
        let pool = SqlitePool::new(&config.cache_db_connection).await?;
        let config_hash = io::sha256_bytes(&bincode::serialize(config)?);
        let db = CacheDB { pool, config_hash };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<(), CookingError> {
        /*sqlx::query(
            r#"
                CREATE TABLE IF NOT EXISTS 
                    cooked_sources (
                        config_hash TEXT NOT NULL,
                        source_id TEXT NOT NULL,
                        source_hash TEXT NOT NULL,
                        cooked_url TEXT NOT NULL );
                CREATE UNIQUE INDEX IF NOT EXISTS cooked_sources_source_id ON cooked_sources(source_id); 
                CREATE INDEX IF NOT EXISTS cooked_sources_source_hash ON cooked_sources(cooked_url); 
                CREATE INDEX IF NOT EXISTS cooked_sources_cooked_url ON cooked_sources(cooked_url);
            "#,
        )
        .execute(&self.pool)
        .await?;*/
        Ok(())
    }

    pub fn hash_source(&self, data: &[&[u8]]) -> String {
        io::sha256_multiple_bytes(data)
    }

    /// Remove all entries not matching the current config
    /*pub async fn purge_config(&self) -> Result<(), CookingError> {
        sqlx::query("DELETE FROM cooked_sources WHERE config_hash != ?")
            .bind(&self.config_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove all the cached information
    pub async fn clear(&self) -> Result<(), CookingError> {
        sqlx::query(
            r#"
                BEGIN TRANSACTION;
                DELETE FROM cooked_sources;
                COMMIT
            "#,
        )
        .bind(&self.config_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_info(&self, source_id: &str) -> Result<Option<SourceCacheEntry>, CookingError> {
        let info = sqlx::query_as::<_, SourceCacheEntry>(
            r#"
                SELECT source_url, source_hash, cooked_url
                    FROM cooked_sources
                    WHERE config_hash = ? and source_url = ?")
            "#,
        )
        .bind(&self.config_hash)
        .bind(&source_url)
        .fetch_optional(&self.pool)
        .await?;
        Ok(info)
    }

    pub async fn get_cooked_urls(&self, source_urls: &[&str]) -> Result<Vec<String>, CookingError> {
        let query = {
            let in_clause = source_urls
                .iter()
                .map(|x| format!("'{}'", x))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"
                    SELECT cooked_url
                        FROM cooked_sources
                        WHERE source_url IN ({})
                "#,
                in_clause
            )
        };
        //log::info!("query: {}", query);
        let entries = sqlx::query_as::<_, (String,)>(&query)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|x| x.0)
            .collect();
        Ok(entries)
    }

    pub async fn get_source_urls(&self, cooked_urls: &[&str]) -> Result<Vec<String>, CookingError> {
        let mut source_urls = Vec::new();
        for cooked_url in cooked_urls {
            let entry = sqlx::query_as::<_, (String,)>(
                r#"
                    SELECT source_url
                        FROM cooked_sources
                        WHERE cooked_url = ?
                "#,
            )
            .bind(cooked_url)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CookingError::Other(format!("Missing source for {}. Cook the root.", cooked_url)))?;
            source_urls.push(entry.0);
        }
        Ok(source_urls)
    }

    pub async fn get_all_infos(&self) -> Result<Vec<SourceCacheEntry>, CookingError> {
        let entries = sqlx::query_as::<_, SourceCacheEntry>(
            r#"
                SELECT *
                    FROM cooked_sources
                    WHERE config_hash = ?;
                    "#,
        )
        .bind(&self.config_hash)
        .fetch_all(&self.pool)
        .await?;
        Ok(entries)
    }

    pub async fn clear_info(&self, source_url: &str) -> Result<(), CookingError> {
        sqlx::query("DELETE FROM cooked_sources WHERE config_hash = ? and source_url = ?")
            .bind(&self.config_hash)
            .bind(source_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }*/
    
    pub async fn set_info(
        &self,
        source_url: &Url,
        source_hash: &str,
        dependency: &Dependency,
    ) -> Result<(), CookingError> {
        Ok(())
        /*sqlx::query(
            r#"
                INSERT OR REPLACE INTO cooked_sources (config_hash, source_url, source_hash, cooked_url)
                VALUES (?,?,?,?)
            "#,
        )
        .bind(&self.config_hash)
        .bind(&source_url)
        .bind(&source_hash)
        .bind(&cooked_url)
        .execute(&self.pool)
        .await?;
        Ok(())*/
    }
}
