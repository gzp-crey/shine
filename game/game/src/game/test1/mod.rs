mod source;
pub use self::source::*;
mod scene;
use self::scene::*;

use crate::{
    app::{App, AppError, GameFuture, GameLifecycle, GameSource},
    assets::{AssetError, Url},
    World,
};
//use shine_ecs::resources::Resources;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Test1Type {
    Test1,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Test1 {
    #[serde(rename = "type")]
    pub ty: Test1Type,
    pub pipeline: String,
}

impl Test1 {
    pub async fn load_into_app(app: &mut App, url: &Url) -> Result<(), AppError> {
        let game = {
            let assetio = app.world.asset_io()?;
            let data = assetio.download_binary(url).await?;
            bincode::deserialize::<Test1>(&data).map_err(|err| AssetError::load_failed(&url, err))?
        };
        app.init_game(game).await
    }
}

impl GameSource for Test1 {
    fn build<'a>(self) -> Result<Box<dyn GameLifecycle>, AppError> {
        Ok(Box::new(self))
    }
}

impl GameLifecycle for Test1 {
    fn create<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            log::info!("Adding test1 scene to the world");

            world
                .resources
                .quick_insert(TestScene::new(&self))
                .map_err(|err| AppError::plugin("test1", err))?;

            /*{
                let mut render_schedule = Schedule::default();
                render_schedule.schedule(render_system(claim));
                world.add_stage("render", render_schedule);
            }*/

            Ok(())
        })
    }

    fn destroy<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            log::info!("Removing test1 scene from the world");

            world.clear_stages();
            let _ = world.resources.remove::<TestScene>();

            Ok(())
        })
    }
}
