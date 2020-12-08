use crate::assets::{AssetError, UniformSemantic, VertexBufferLayout, VertexSemantic};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PipelineUniformScope {
    Auto,
    Global,
    Local,
}

impl PipelineUniformScope {
    pub fn bind_location(self) -> u32 {
        match self {
            PipelineUniformScope::Auto => 0,
            PipelineUniformScope::Global => 1,
            PipelineUniformScope::Local => 2,
        }
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

/// List of uniform requirements of the pipeline for each share stage
#[derive(Debug)]
pub struct PipelineUniformLayout(Vec<(PipelineUniform, wgpu::ShaderStage)>);

impl PipelineUniformLayout {
    pub fn merge_from_stages(layouts: &[(&[PipelineUniform], wgpu::ShaderStage)]) -> Result<Self, AssetError> {
        let mut merged: HashMap<u32, (PipelineUniform, wgpu::ShaderStage)> = Default::default();
        for (layout, stage) in layouts.iter() {
            for uniform in layout.iter() {
                if let Some(u) = merged.get_mut(&uniform.location()) {
                    if u.0 != *uniform {
                        return Err(AssetError::Content(format!(
                            "Incompatible uniform binding at {} location: {:?} vs {:?}",
                            uniform.location(),
                            u,
                            (uniform, stage)
                        )));
                    }
                    u.1 |= *stage;
                } else {
                    merged.insert(uniform.location(), (uniform.clone(), *stage));
                }
            }
        }

        let mut result: Vec<_> = merged.values().cloned().collect();
        result.sort_by(|a, b| (a.0).0.partial_cmp(&(b.0).0).unwrap());
        Ok(PipelineUniformLayout(result))
    }

    pub fn iter(&self) -> impl Iterator<Item = &(PipelineUniform, wgpu::ShaderStage)> {
        self.0.iter()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: Vec<PipelineAttribute>,
    //pub auto_uniforms: Vec<PipelineUniform>,
    //pub global_uniforms: Vec<PipelineUniform>,
    //pub local_uniforms: Vec<PipelineUniform>,
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
    //pub auto_uniforms: Vec<PipelineUniform>,
    //pub global_uniforms: Vec<PipelineUniform>,
    //pub local_uniforms: Vec<PipelineUniform>,
}

/// Deserialized pipeline data
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PipelineDescriptor {
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub vertex_stage: VertexStage,
    pub fragment_stage: FragmentStage,
}

impl PipelineDescriptor {
    /*pub fn get_uniform_layout(&self, scope: PipelineUniformScope) -> Result<PipelineUniformLayout, AssetError> {
        let (vs_uniforms, fs_uniforms) = match scope {
            PipelineUniformScope::Auto => (&self.vertex_stage.auto_uniforms, &self.fragment_stage.auto_uniforms),
            PipelineUniformScope::Global => (&self.vertex_stage.global_uniforms, &self.fragment_stage.global_uniforms),
            PipelineUniformScope::Local => (&self.vertex_stage.local_uniforms, &self.fragment_stage.local_uniforms),
        };

        PipelineUniformLayout::merge_from_stages(&[
            (vs_uniforms, wgpu::ShaderStage::VERTEX),
            (fs_uniforms, wgpu::ShaderStage::FRAGMENT),
        ])
    }*/
}

/*#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineStateDescriptor {
    pub color_states: Vec<wgpu::ColorStateDescriptor>,
    pub depth_state: Option<wgpu::DepthStencilStateDescriptor>,
}*/
