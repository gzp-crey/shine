use crate::assets::PipelineDescriptor;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CookedPipeline {
    pub descriptor: PipelineDescriptor,
}
