use crate::assets::{AssetError, UniformSemantic, VertexBufferLayout, VertexSemantic};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

/// Vertex attribute requirement of the pipeline
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PipelineAttribute(u32, VertexSemantic, wgpu::VertexFormat);

impl PipelineAttribute {
    pub fn new(location: u32, semantic: VertexSemantic, format: wgpu::VertexFormat) -> PipelineAttribute {
        PipelineAttribute(location, semantic, format)
    }

    pub fn location(&self) -> u32 {
        self.0
    }

    pub fn semantic(&self) -> &VertexSemantic {
        &self.1
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        self.2
    }
}

/// Uniform requirement of the pipeline
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PipelineUniform(u32, UniformSemantic);

impl PipelineUniform {
    pub fn new(location: u32, semantic: UniformSemantic) -> PipelineUniform {
        PipelineUniform(location, semantic)
    }

    pub fn location(&self) -> u32 {
        self.0
    }

    pub fn semantic(&self) -> &UniformSemantic {
        &self.1
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: Vec<PipelineAttribute>,
    pub uniforms: Vec<(u32, Vec<PipelineUniform>)>,
}

impl VertexStage {
    pub fn check_vertex_format(target: wgpu::VertexFormat, source: wgpu::VertexFormat) -> Result<(), AssetError> {
        if target != source {
            Err(AssetError::Content(format!(
                "Vertex attribute format missmacth, target:{:?}, source:{:?}",
                target, source
            )))
        } else {
            Ok(())
        }
    }

    pub fn check_vertex_layouts(&self, vertex_layouts: &[VertexBufferLayout]) -> Result<(), AssetError> {
        // check vertex attribute source duplication and the format compatibility
        let mut semantics = HashSet::new();
        for layout in vertex_layouts {
            for va in &layout.attributes {
                if let Some(pa) = self.attributes.iter().find(|pa| pa.semantic() == va.semantic()) {
                    Self::check_vertex_format(pa.format(), va.format())?;
                    if !semantics.insert(pa.semantic().clone()) {
                        return Err(AssetError::Content(format!(
                            "{:?} attribute defined multiple times",
                            pa.semantic()
                        )));
                    }
                }
            }
        }
        log::trace!("vertex_layouts: {:?}", vertex_layouts);
        log::trace!("pipeline attributes: {:?}", self.attributes);
        log::trace!("Mapped semmantics: {:?}", semantics);

        // check if all the pipeline attributes are covered
        for pa in &self.attributes {
            if !semantics.contains(pa.semantic()) {
                return Err(AssetError::Content(format!(
                    "Missing attribute for {:?}",
                    pa.semantic()
                )));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FragmentStage {
    pub shader: String,
    pub uniforms: Vec<(u32, Vec<PipelineUniform>)>,
}

/// Deserialized pipeline data
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PipelineDescriptor {
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub vertex_stage: VertexStage,
    pub fragment_stage: FragmentStage,
}

impl PipelineDescriptor {
    pub fn get_uniform_layout(&self) -> Result<PipelineUniformLayout, AssetError> {
        // store the uniform info for each location for each group to check validity
        let mut check: HashMap<u32, (UniformSemantic, u32, wgpu::ShaderStage)> = Default::default();
        let mut merged: HashMap<u32, HashMap<u32, (UniformSemantic, wgpu::ShaderStage)>> = Default::default();

        for (stage_uniforms, stage) in &[
            (&self.vertex_stage.uniforms, wgpu::ShaderStage::VERTEX),
            (&self.fragment_stage.uniforms, wgpu::ShaderStage::FRAGMENT),
        ] {
            for (binding_group_id, stage_uniforms) in stage_uniforms.iter() {
                for uniform in stage_uniforms.iter() {
                    //check consisentcy
                    if let Some(u) = check.get_mut(&uniform.location()) {
                        if u.1 != *binding_group_id {
                            return Err(AssetError::Content(format!(
                                "mismatching uniform group {:?}/{}/{} ({:?} vs {:?})",
                                stage,
                                binding_group_id,
                                uniform.location(),
                                u.1,
                                binding_group_id
                            )));
                        }
                        if u.0 != *uniform.semantic() {
                            return Err(AssetError::Content(format!(
                                "Incompatible uniform semantic for {:?}/{}/{} ({:?} vs {:?})",
                                stage,
                                binding_group_id,
                                uniform.location(),
                                u.0,
                                uniform.semantic()
                            )));
                        }
                        if u.2 & *stage == *stage {
                            return Err(AssetError::Content(format!(
                                "Duplicate uniform binding for {:?}/{}/{}",
                                stage,
                                binding_group_id,
                                uniform.location(),
                            )));
                        }
                        u.2 |= *stage;
                    } else {
                        check.insert(
                            uniform.location(),
                            (uniform.semantic().clone(), *binding_group_id, *stage),
                        );
                    }

                    //update layout
                    let binding_group = merged.entry(*binding_group_id).or_insert_with(Default::default);
                    let binding_group_entry = binding_group
                        .entry(uniform.location())
                        .or_insert_with(|| (uniform.semantic().clone(), *stage));
                    binding_group_entry.1 |= *stage;
                }
            }
        }

        // convert map of map into vec of vec
        let mut result = PipelineUniformLayout::default();

        let binding_group_count = merged.keys().map(|&x| x as usize + 1).max().unwrap_or(0);
        result.resize_with(binding_group_count, Default::default);

        for (binding_group_id, mut merge_group) in merged.drain() {
            let binding_group_id = binding_group_id as usize;
            result[binding_group_id] = merge_group
                .drain()
                .map(|(loc, (sem, stage))| (PipelineUniform::new(loc, sem), stage))
                .collect()
        }

        Ok(result)
    }
}

#[derive(Default, Debug, Clone)]
pub struct PipelineUniformLayout(Vec<Vec<(PipelineUniform, wgpu::ShaderStage)>>);

impl Deref for PipelineUniformLayout {
    type Target = Vec<Vec<(PipelineUniform, wgpu::ShaderStage)>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PipelineUniformLayout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct PipelineStateDescriptor {
    pub color_states: Vec<wgpu::ColorStateDescriptor>,
    pub depth_state: Option<wgpu::DepthStencilStateDescriptor>,
}
