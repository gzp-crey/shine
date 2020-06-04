use crate::{Config, CookingError};
use shine_game::assets::{io::HashableContent, AssetError, AssetIO, Url, UrlError};
use sqlx::{
    self,
    executor::Executor,
    postgres::{PgPool, PgQueryAs},
};
use std::sync::Arc;

pub enum AssetNaming {
    Hard,
    SoftScheme(String),
}

#[derive(Debug, sqlx::FromRow)]
pub struct Dependency {
    pub parent_url: String,
    pub child_url: String,
    pub is_soft: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TargetDependency {
    child_url: String,
    is_soft: bool,
}

impl TargetDependency {
    pub fn soft(child_url: &str) -> TargetDependency {
        TargetDependency {
            child_url: child_url.to_owned(),
            is_soft: true,
        }
    }

    pub fn hard(child_url: &str) -> TargetDependency {
        TargetDependency {
            child_url: child_url.to_owned(),
            is_soft: false,
        }
    }

    pub fn is_soft(&self) -> bool {
        self.is_soft
    }

    pub fn is_hard(&self) -> bool {
        !self.is_soft
    }

    pub fn url(&self) -> &str {
        &self.child_url
    }

    pub fn to_url(&self) -> Result<Url, UrlError> {
        Url::parse(&self.child_url)
    }
}

//Manage local sources to speed up compilation
#[derive(Clone)]
pub struct TargetDB {
    pool: PgPool,
    io: Arc<AssetIO>,
}

impl TargetDB {
    pub async fn new(config: &Config) -> Result<TargetDB, CookingError> {
        let pool = PgPool::new(&config.target_db_connection).await?;
        let io = Arc::new(AssetIO::new(config.target_virtual_schemes.clone())?);
        let db = TargetDB { pool, io };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<(), CookingError> {
        println!("1");
        (&self.pool)
            .execute(
                r#"
                CREATE TABLE IF NOT EXISTS
                    dependencies (
                        parent_url TEXT NOT NULL,
                        child_url TEXT NOT NULL,
                        is_soft BOOLEAN );
                CREATE UNIQUE INDEX IF NOT EXISTS dependencies_parent_url_child_url ON dependencies(parent_url, child_url);
                CREATE INDEX IF NOT EXISTS dependencies_child_url ON dependencies(child_url);
                CREATE INDEX IF NOT EXISTS dependencies_parent_url ON dependencies(parent_url);

                CREATE TABLE IF NOT EXISTS 
                    asset_sources (
                        source_id TEXT NOT NULL,
                        cooked_url TEXT NOT NULL );
                CREATE UNIQUE INDEX IF NOT EXISTS asset_sources_source_id ON asset_sources(source_id); 
                CREATE INDEX IF NOT EXISTS asset_sources_cooked_url ON asset_sources(cooked_url);
            "#,
            )
            .await?;
        println!("2");
        Ok(())
    }

    async fn update_dependencies_and_source_id(
        &self,
        target_url: &str,
        source_id: &str,
        dependencies: Vec<TargetDependency>,
    ) -> Result<(), CookingError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM dependencies where parent_url = $1")
            .bind(target_url)
            .execute(&mut tx)
            .await?;

        sqlx::query(
            r#"
                INSERT INTO asset_sources(source_id, cooked_url)
                    VALUES($1, $2)
                ON CONFLICT (source_id)
                    DO UPDATE SET cooked_url = $2
            "#,
        )
        .bind(target_url)
        .bind(&source_id)
        .execute(&self.pool)
        .await?;

        for dep in dependencies {
            sqlx::query(
                r#"
                    INSERT INTO dependencies(parent_url, child_url, is_soft) 
                    VALUES ($1,$2,$3)
                "#,
            )
            .bind(target_url)
            .bind(&dep.child_url)
            .bind(&dep.is_soft)
            .execute(&mut tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_all_dependencies(&self) -> Result<Vec<Dependency>, CookingError> {
        let dependencies = sqlx::query_as::<_, Dependency>("SELECT * FROM dependencies")
            .fetch_all(&self.pool)
            .await?;
        Ok(dependencies)
    }

    // Return the affected root parents with hard dependency. It recursively travels all the parent from the given child
    // following only the hard links and rturn the root elements.
    pub async fn get_affected_roots(&self, child_urls: &[&str]) -> Result<Vec<String>, CookingError> {
        log::info!("child_urls: {:?}", child_urls);
        let roots = sqlx::query_as::<_, (String,)>(
            r#"
            WITH RECURSIVE roots AS (
                SELECT child_url, parent_url
                    FROM dependencies
                    WHERE child_url = ANY($1)
                UNION
                    SELECT d.child_url, d.parent_url
                        FROM dependencies d 		
                        INNER JOIN roots r ON d.child_url = r.parent_url
                        WHERE NOT d.is_soft
            ) SELECT DISTINCT(parent_url) 
                FROM roots r1 
                WHERE NOT EXISTS (
                    SELECT * 
                        FROM roots r2 
                        WHERE r1.parent_url = r2.child_url
                )
            "#,
        )
        .bind(child_urls)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|x| x.0)
        .collect();
        Ok(roots)
    }

    async fn upload_binary_content(
        &self,
        source_id: &str,
        asset_url: &Url,
        naming: AssetNaming,
        content: &[u8],
    ) -> Result<TargetDependency, AssetError> {
        let target_dependency = match naming {
            AssetNaming::Hard => {
                let hashed_path = content.hashed_path();
                let target_id = format!("{}.{}", hashed_path, asset_url.extension());
                TargetDependency::hard(&format!("hash://{}", target_id))
            }
            AssetNaming::SoftScheme(scheme) => TargetDependency::soft(&format!("{}://{}", scheme, source_id)),
        };
        self.io.upload_binary(&target_dependency.to_url()?, &content).await?;
        Ok(target_dependency)
    }

    pub async fn upload_cooked_binary(
        &self,
        asset_id: &AssetId,
        asset_url: &Url,
        naming: AssetNaming,
        content: &[u8],
        dependencies: Vec<TargetDependency>,
    ) -> Result<TargetDependency, CookingError> {
        let target_dependency = self
            .upload_binary_content(asset_id.as_str(), asset_url, naming, content)
            .await?;
        self.update_dependencies_and_source_id(target_dependency.url(), asset_id.as_str(), dependencies)
            .await?;
        Ok(target_dependency)
    }
}
