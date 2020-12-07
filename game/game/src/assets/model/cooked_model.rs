use crate::assets::MeshData;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct CookedModel {
    pub meshes: Vec<MeshData>,
}
