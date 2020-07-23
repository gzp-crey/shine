use crate::assets::{AssetError, Uniform, VertexBufferLayout, VertexSemantic};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// Supported shader types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

impl FromStr for ShaderType {
    type Err = AssetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fs_spv" => Ok(ShaderType::Fragment),
            "vs_spv" => Ok(ShaderType::Vertex),
            "cs_spv" => Ok(ShaderType::Compute),
            _ => Err(AssetError::UnsupportedFormat(s.to_owned())),
        }
    }
}

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
pub struct PipelineUniform(u32, Uniform);

impl PipelineUniform {
    pub fn new(location: u32, uniform: Uniform) -> PipelineUniform {
        PipelineUniform(location, uniform)
    }

    pub fn location(&self) -> u32 {
        self.0
    }

    pub fn uniform(&self) -> &Uniform {
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
    pub uniforms: Vec<Vec<PipelineUniform>>,
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
    pub uniforms: Vec<Vec<PipelineUniform>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Blending {
    Replace,
}

/// Deserialized pipeline data
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PipelineDescriptor {
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub vertex_stage: VertexStage,
    pub fragment_stage: FragmentStage,
    pub color_stage: Blending,
    //depth_stencile_stage:
}

impl PipelineDescriptor {
    pub fn get_uniform_layout(&self, group: usize) -> Result<PipelineUniformLayout, AssetError> {
        PipelineUniformLayout::merge_from_stages(&[
            (&self.vertex_stage.uniforms[group], wgpu::ShaderStage::VERTEX),
            (&self.fragment_stage.uniforms[group], wgpu::ShaderStage::FRAGMENT),
        ])
    }
}
