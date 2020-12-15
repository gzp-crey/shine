use crate::{Config, CookerError};
use shine_game::assets::{
    cooker::{CookingError, Naming},
    AssetIO, AssetId, ContentHash, Url,
};
use sqlx::PgPool;

//Manage local sources to speed up compilation
#[derive(Clone)]
pub struct TargetDB {
    pool: Option<PgPool>,
    asset_io: AssetIO,
    scopes: Vec<AssetId>,
}

impl TargetDB {
    pub async fn new(config: &Config) -> Result<TargetDB, CookerError> {
        log::info!("Connecting to db...");
        let pool = if let Some(conn) = &config.target_db_connection {
            Some(PgPool::new(&conn).await?)
        } else {
            None
        };
        let asset_io = AssetIO::new(config.target_virtual_schemes.clone())?;
        let db = TargetDB {
            pool,
            asset_io,
            scopes: Vec::new(),
        };
        //db.init().await?;
        log::info!("Db done.");
        Ok(db)
    }

    pub fn create_scope(&self, scope: AssetId) -> TargetDB {
        TargetDB {
            pool: self.pool.clone(),
            asset_io: self.asset_io.clone(),
            scopes: self.scopes.iter().cloned().chain(Some(scope)).collect(),
        }
    }

    pub async fn upload_binary_content(
        &self,
        source_id: AssetId,
        _source_hash: ContentHash,
        naming: Naming,
        cooked_content: &[u8],
    ) -> Result<Url, CookingError> {
        if naming.is_hard() && self.scopes.is_empty() {
            return Err(CookingError::from_str(
                &source_id,
                format!("Hard naming without an asset scope"),
            ));
        }

        let cooked_hash = ContentHash::from_bytes(cooked_content);
        let target_url = naming
            .to_url(&source_id, &cooked_hash)
            .map_err(|err| CookingError::from_err(&source_id, err))?;
        self.asset_io
            .upload_binary(&target_url, &cooked_content)
            .await
            .map_err(|err| CookingError::from_err(&source_id, err))?;

        // update dependency of owner_id
        Ok(target_url)
    }
}
