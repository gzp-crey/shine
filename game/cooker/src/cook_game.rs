use crate::Context;
use serde::Deserialize;
use shine_game::{
    assets::{
        cooker::{CookingError, Naming},
        AssetId, Url,
    },
    game::test1,
};

#[derive(Deserialize)]
struct GameType {
    #[serde(rename = "type")]
    ty: String,
}

impl Context {
    pub async fn cook_game(&self, source_id: AssetId) -> Result<Url, CookingError> {
        let source_url = source_id
            .to_url(&self.source_root)
            .map_err(|err| CookingError::from_err(&source_id, err))?;

        let game_data = self
            .source_io
            .download_binary(&source_url)
            .await
            .map_err(|err| CookingError::from_err(&source_id, err))?;

        if let Ok((source, source_hash)) = test1::Source::load_from_data(&source_id, &source_url, &game_data).await {
            log::debug!("[{}] Found game type: {:?}", source_url, source.test.ty);

            let cooked = source.cook(self.create_scope(source_id.clone())).await?;
            let cooked_content = bincode::serialize(&cooked).map_err(|err| CookingError::from_err(&source_id, err))?;

            log::debug!("[{}] Uploading...", source_url);
            let cooked_url = self
                .target_io
                .upload_binary_content(source_id, source_hash, Naming::soft("game", "g1"), &cooked_content)
                .await?;
            Ok(cooked_url)
        } else if let Ok(game) = serde_json::from_slice::<GameType>(&game_data) {
            Err(CookingError::from_str(
                source_id,
                format!("Unknown game type: {}", game.ty),
            ))
        } else {
            Err(CookingError::from_str(source_id, "Invalid game file"))
        }
    }
}
