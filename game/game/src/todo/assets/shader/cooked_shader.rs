use crate::assets::{AssetId, ShaderType, SourceShader};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CookedShader {
    pub ty: ShaderType,
    pub binary: Vec<u8>,
}

#[cfg(feature = "cook")]
mod cooker {
    use super::*;
    use crate::assets::CookingError;
    use shaderc;

    impl CookedShader {
        #[cfg(feature = "cook")]
        pub async fn cook(id: &AssetId, src: &SourceShader) -> Result<Self, CookingError> {
            log::debug!("[{}] Compiling...", id.as_str());
            log::trace!("[{}] Source ({:?}):\n{}", id.as_str(), src.ty, src.source);

            let ty = match src.ty {
                ShaderType::Fragment => shaderc::ShaderKind::Fragment,
                ShaderType::Vertex => shaderc::ShaderKind::Vertex,
                ShaderType::Compute => shaderc::ShaderKind::Compute,
            };

            let mut compiler = shaderc::Compiler::new().unwrap();
            let options = shaderc::CompileOptions::new().unwrap();
            let compiled_artifact = compiler
                .compile_into_spirv(&src.source, ty, id.as_str(), "main", Some(&options))
                .map_err(|err| CookingError::Cook {
                    content_id: id.as_str().to_owned(),
                    source: err.into(),
                })?;

            Ok(CookedShader {
                ty: src.ty,
                binary: compiled_artifact.as_binary_u8().to_owned(),
            })
        }
    }
}
