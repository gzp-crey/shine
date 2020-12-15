use std::sync::Arc;
use shine_game::assets::{AssetIO, Url, CookShader, PipelineCooker, ShaderCooker};
use crate::TargetDB;


#[derive(Clone)]
pub struct Context {
    // all asset source is located in the root
    pub source_root: Url,
    pub source_io: Arc<AssetIO>,
    pub target_db: TargetDB,
}


impl<'a> ShaderCooker<'a> for Context {
    type ShaderFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_shader(&self, _sh: ShaderType, id: AssetId) -> Self::ShaderFuture {
        Box::pin(
            async move {
               let dep = cook_shader::cook_shader(self, id).await
                    .map_err(|err| CookingError::from_err(id.as_str(), err))?;
                Ok(dep.cooked_url)
            }
        )
    }
}

//pub trait PipelineCooker<'a>: ShaderCooker<'a> {