use crate::{Config, CookerError, Dependency, TargetNaming};
use shine_game::assets::{io::HashableContent, AssetIO, AssetId, Url};
use sqlx::{
    self,
    executor::Executor,
    postgres::{PgPool, PgQueryAs},
};
use std::sync::Arc;

//Manage local sources to speed up compilation
#[derive(Clone)]
pub struct TargetDB {
    pool: Option<PgPool>,
    io: Arc<AssetIO>,
}

impl TargetDB {
    pub async fn new(config: &Config) -> Result<TargetDB, CookerError> {
        log::info!("Connecting to db...");
        let pool = if let Some(conn) = &config.target_db_connection {
            Some(PgPool::new(&conn).await?)
        } else {
            None
        };
        let io = Arc::new(AssetIO::new(config.target_virtual_schemes.clone())?);
        let db = TargetDB { pool, io };
        db.init().await?;
        log::info!("Db done.");
        Ok(db)
    }

    async fn init(&self) -> Result<(), CookerError> {
        if let Some(pool) = &self.pool {
            (&*pool)
            .execute(
                r#"
                    CREATE TABLE IF NOT EXISTS
                        source_dependencies (
                            parent TEXT NOT NULL,
                            child TEXT NOT NULL,
                            is_soft BOOLEAN );
                    CREATE UNIQUE INDEX IF NOT EXISTS source_dependencies_parent_child ON source_dependencies(parent, child);
                    CREATE INDEX IF NOT EXISTS source_dependencies_child ON source_dependencies(child);
                    CREATE INDEX IF NOT EXISTS source_dependencies_parent ON source_dependencies(parent);

                    CREATE TABLE IF NOT EXISTS
                        cooked_dependencies (
                            parent TEXT NOT NULL,
                            child TEXT NOT NULL,
                            is_soft BOOLEAN );
                    CREATE UNIQUE INDEX IF NOT EXISTS cooked_dependencies_parent_child ON cooked_dependencies(parent, child);
                    CREATE INDEX IF NOT EXISTS cooked_dependencies_child ON cooked_dependencies(child);
                    CREATE INDEX IF NOT EXISTS cooked_dependencies_parent ON cooked_dependencies(parent);

                    CREATE TABLE IF NOT EXISTS 
                        sources (
                            source_id TEXT NOT NULL,
                            source_hash TEXT NOT NULL,
                            cooked_url TEXT NOT NULL );
                    CREATE UNIQUE INDEX IF NOT EXISTS sources_source_id ON sources(source_id); 
                    CREATE INDEX IF NOT EXISTS sources_cooked_url ON sources(cooked_url);
                "#,
            )
            .await?;
        }
        Ok(())
    }

    async fn update_dependencies(
        &self,
        parent: &Dependency,
        _source_url: &Url,
        source_hash: String,
        dependencies: Vec<Dependency>,
    ) -> Result<(), CookerError> {
        if let Some(pool) = &self.pool {
            let mut tx = pool.begin().await?;
            let source_id = parent.source_id();

            sqlx::query("DELETE FROM source_dependencies where parent = $1")
                .bind(source_id.as_str())
                .execute(&mut tx)
                .await?;
            sqlx::query("DELETE FROM cooked_dependencies where parent = $1")
                .bind(parent.cooked_url().as_str())
                .execute(&mut tx)
                .await?;

            for dep in dependencies {
                sqlx::query(
                    r#"
                        INSERT INTO source_dependencies(parent, child, is_soft)
                        VALUES ($1,$2,$3)
                    "#,
                )
                .bind(source_id.as_str())
                .bind(dep.source_id().as_str())
                .bind(dep.is_soft)
                .execute(&mut tx)
                .await?;

                sqlx::query(
                    r#"
                        INSERT INTO cooked_dependencies(parent, child, is_soft)
                        VALUES ($1,$2,$3)
                    "#,
                )
                .bind(parent.cooked_url().as_str())
                .bind(dep.cooked_url().as_str())
                .bind(dep.is_soft)
                .execute(&mut tx)
                .await?;
            }

            sqlx::query(
                r#"
                    INSERT INTO sources(source_id, cooked_url, source_hash)
                        VALUES($1, $2, $3)
                    ON CONFLICT (source_id)
                        DO UPDATE SET cooked_url = $2, source_hash = $3
                "#,
            )
            .bind(source_id.as_str())
            .bind(parent.cooked_url().as_str())
            .bind(source_hash)
            .execute(pool)
            .await?;

            tx.commit().await?;
        }
        Ok(())
    }

    // Return the affected root parents with hard dependency. It recursively travels all the parent from the given children
    // following only the hard links and return the root elements. The response also contains the unknow resources.
    pub async fn get_affected_roots(&self, asset_ids: &[AssetId]) -> Result<Vec<AssetId>, CookerError> {
        log::info!("asset_ids: {:?}", asset_ids);

        if let Some(pool) = &self.pool {
            let asset_ids_str = asset_ids.iter().map(|x| x.as_str()).collect::<Vec<_>>();
            let roots = sqlx::query_as::<_, (String,)>(
                r#"
                ( 
                    -- collect all the roots with a soft parent (the top-most roots are not part of this recursive query)
                    WITH RECURSIVE roots AS (
                        SELECT child, parent
                            FROM source_dependencies
                            WHERE child = ANY($1)
                        UNION
                            SELECT d.child, d.parent
                                FROM source_dependencies d 		
                                INNER JOIN roots r ON d.child = r.parent
                                WHERE NOT d.is_soft
                    ) SELECT DISTINCT(parent)
                        FROM roots r1 
                        WHERE NOT EXISTS (
                            SELECT * 
                                FROM roots r2 
                                WHERE r1.parent = r2.child 
                        )
                )
                UNION
                (
                    -- collect unknown ids as roots
                    -- and collect the topmost parent those were excluded from the previous query
                    SELECT parent
                    FROM (
                        SELECT unnest($1) as parent
                    ) AS seeds 
                    WHERE NOT EXISTS (SELECT * from sources WHERE source_id = seeds.parent)
                        OR EXISTS (SELECT * from source_dependencies d WHERE d.parent = seeds.parent AND d.is_soft)
                )
            "#,
            )
            .bind(&asset_ids_str)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|x| AssetId::new(&x.0))
            .collect::<Result<Vec<_>, _>>()?;
            Ok(roots)
        } else {
            Ok(asset_ids.to_vec())
        }
    }

    async fn upload_binary_content(
        &self,
        asset_id: &AssetId,
        cooked_naming: TargetNaming,
        cooked_content: &[u8],
    ) -> Result<Dependency, CookerError> {
        let target_dependency = match cooked_naming {
            TargetNaming::Hard(scheme, ext) => {
                let hashed_path = cooked_content.content_hash_path();
                let ext = ext.unwrap_or_else(|| asset_id.extension().to_owned());
                let url = Url::parse(&format!("hash-{}://{}.{}", scheme, hashed_path, ext))?;
                Dependency::hard(asset_id.clone(), url)
            }
            TargetNaming::Soft(scheme, ext) => {
                let ext = ext.unwrap_or_else(|| asset_id.extension().to_owned());
                let url = Url::parse(&format!("{}://{}", scheme, asset_id.as_str()))?.set_extension(&ext)?;
                Dependency::soft(asset_id.clone(), url)
            }
        };

        self.io
            .upload_binary(&target_dependency.cooked_url(), &cooked_content)
            .await?;
        Ok(target_dependency)
    }

    pub async fn upload_cooked_binary(
        &self,
        asset_id: &AssetId,
        source_url: &Url,
        source_hash: String,
        cooked_naming: TargetNaming,
        cooked_content: &[u8],
        dependencies: Vec<Dependency>,
    ) -> Result<Dependency, CookerError> {
        let target_dependency = self
            .upload_binary_content(asset_id, cooked_naming, cooked_content)
            .await?;
        self.update_dependencies(&target_dependency, source_url, source_hash, dependencies)
            .await?;
        Ok(target_dependency)
    }
}
