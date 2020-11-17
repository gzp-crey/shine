use crate::assets::ShaderType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CookedShader {
    pub ty: ShaderType,
    pub binary: Vec<u8>,
}
