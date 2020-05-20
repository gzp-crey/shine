use crate::assets::{AssetError, VertexBufferLayout};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Hash, Deserialize, PartialEq, Eq)]
pub enum VertexSemantic {
    Position,
    Color(u8),
    TexCoord(u8),
    Normal,
    Tangent,
    Custom(String),
}

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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum UniformSemantic {
    ModelView,

    Diffuse,
    Normal,

    Custom(String),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum UniformFormat {
    Sampler,
    Texture,
    //Binary,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PipelineUniform(u32, UniformSemantic, UniformFormat);

impl PipelineUniform {
    pub fn new(location: u32, semantic: UniformSemantic, format: UniformFormat) -> PipelineUniform {
        PipelineUniform(location, semantic, format)
    }

    pub fn location(&self) -> u32 {
        self.0
    }

    pub fn semantic(&self) -> &UniformSemantic {
        &self.1
    }

    pub fn format(&self) -> &UniformFormat {
        &self.2
    }
}

fn merge_uniforms(
    layouts: &[(&[PipelineUniform], wgpu::ShaderStage)],
) -> Result<Vec<(PipelineUniform, wgpu::ShaderStage)>, AssetError> {
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
    Ok(result)
}

fn create_uniform_bindings(
    layout: &Vec<(PipelineUniform, wgpu::ShaderStage)>,
) -> Result<Vec<wgpu::BindGroupLayoutEntry>, AssetError> {
    let mut descriptor = Vec::new();
    for (uniform, stages) in layout.iter() {
        descriptor.push(match uniform.2 {
            UniformFormat::Texture => wgpu::BindGroupLayoutEntry {
                binding: uniform.location(),
                visibility: *stages,
                ty: wgpu::BindingType::SampledTexture {
                    multisampled: false,
                    dimension: wgpu::TextureViewDimension::D2,
                    component_type: wgpu::TextureComponentType::Uint,
                },
            },
            UniformFormat::Sampler => wgpu::BindGroupLayoutEntry {
                binding: uniform.location(),
                visibility: *stages,
                ty: wgpu::BindingType::Sampler { comparison: false },
            },
        });
    }
    Ok(descriptor)
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: Vec<PipelineAttribute>,
    pub global_uniforms: Vec<PipelineUniform>,
    pub local_uniforms: Vec<PipelineUniform>,
}

impl VertexStage {
    fn check_format(&self, target: wgpu::VertexFormat, source: wgpu::VertexFormat) -> Result<(), AssetError> {
        if target != source {
            Err(AssetError::Content(format!(
                "Vertex attribute format missmacth, target:{:?}, source:{:?}",
                target, source
            )))
        } else {
            Ok(())
        }
    }

    pub fn check_vertex_layouts(&self, vertex_layouts: &Vec<VertexBufferLayout>) -> Result<(), AssetError> {
        // check vertex attribute source duplication and the format compatibility
        let mut semantics = HashSet::new();
        for layout in vertex_layouts {
            for va in &layout.attributes {
                if let Some(pa) = self.attributes.iter().find(|pa| pa.semantic() == va.semantic()) {
                    self.check_format(pa.format(), va.format())?;
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

    pub fn create_attribute_descriptors(
        &self,
        vertex_layouts: &Vec<VertexBufferLayout>,
    ) -> Result<Vec<(wgpu::BufferAddress, Vec<wgpu::VertexAttributeDescriptor>)>, AssetError> {
        let mut descriptors = Vec::new();
        for layout in vertex_layouts {
            let mut attributes = Vec::new();
            for va in &layout.attributes {
                if let Some(pa) = self.attributes.iter().find(|pa| pa.semantic() == va.semantic()) {
                    self.check_format(pa.format(), va.format())?;

                    attributes.push(wgpu::VertexAttributeDescriptor {
                        offset: va.offset(),
                        format: va.format(),
                        shader_location: pa.location(),
                    });
                }
            }

            descriptors.push((layout.stride, attributes));
        }

        Ok(descriptors)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FragmentStage {
    pub shader: String,
    pub global_uniforms: Vec<PipelineUniform>,
    pub local_uniforms: Vec<PipelineUniform>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Blending {
    Replace,
}

/// Compiled pipeline with related binding information
pub struct PipelineBuffer {
    pub pipeline: wgpu::RenderPipeline,
    pub global_binding_group_layout: Option<wgpu::BindGroupLayout>,
    pub local_binding_group_layout: Option<wgpu::BindGroupLayout>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PipelineDescriptor {
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub vertex_stage: VertexStage,
    pub fragment_stage: FragmentStage,
    color_stage: Blending,
    //depth_stencile_stage:
}

impl PipelineDescriptor {
    pub fn get_global_uniform_layout(&self) -> Result<Vec<(PipelineUniform, wgpu::ShaderStage)>, AssetError> {
        merge_uniforms(&[
            (&self.vertex_stage.global_uniforms, wgpu::ShaderStage::VERTEX),
            (&self.fragment_stage.global_uniforms, wgpu::ShaderStage::FRAGMENT),
        ])
    }

    pub fn get_local_uniform_layout(&self) -> Result<Vec<(PipelineUniform, wgpu::ShaderStage)>, AssetError> {
        merge_uniforms(&[
            (&self.vertex_stage.local_uniforms, wgpu::ShaderStage::VERTEX),
            (&self.fragment_stage.local_uniforms, wgpu::ShaderStage::FRAGMENT),
        ])
    }

    pub fn compile(
        &self,
        device: &wgpu::Device,
        color_state_format: wgpu::TextureFormat,
        vertex_layouts: &Vec<VertexBufferLayout>,
        (vs, fs): (&wgpu::ShaderModule, &wgpu::ShaderModule),
    ) -> Result<PipelineBuffer, AssetError> {
        let vertex_buffers = self.vertex_stage.create_attribute_descriptors(&vertex_layouts)?;
        let vertex_buffers: Vec<_> = vertex_buffers
            .iter()
            .map(|(stride, attributes)| wgpu::VertexBufferDescriptor {
                stride: *stride,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &attributes,
            })
            .collect();
        let vertex_state = wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &vertex_buffers,
        };
        log::info!("vertex_state: {:?}", vertex_state);

        let global_binding_group_layout = {
            let bindings = create_uniform_bindings(&self.get_global_uniform_layout()?)?;
            if bindings.is_empty() {
                Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    bindings: &bindings,
                }))
            } else {
                None
            }
        };

        let local_binding_group_layout = {
            let bindings = create_uniform_bindings(&self.get_local_uniform_layout()?)?;
            if bindings.is_empty() {
                Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    bindings: &bindings,
                }))
            } else {
                None
            }
        };        

        let pipeline_layout = {
            let mut bind_group_layouts = Vec::new();
            if let Some(ref binding_group_layout) = global_binding_group_layout {
                bind_group_layouts.push(binding_group_layout);
            }
            if let Some(ref binding_group_layout) = local_binding_group_layout {
                bind_group_layouts.push(binding_group_layout);
            }

            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &bind_group_layouts
            })
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            primitive_topology: self.primitive_topology,

            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: vs,
                entry_point: "main",
            },

            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: fs,
                entry_point: "main",
            }),

            rasterization_state: None,

            color_states: &[wgpu::ColorStateDescriptor {
                format: color_state_format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],

            depth_stencil_state: None,

            vertex_state,

            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Ok(PipelineBuffer {
            pipeline,
            global_binding_group_layout,
            local_binding_group_layout,
        })
    }
}

pub fn foo() {
    use wgpu::VertexFormat as VF;
    use UniformFormat as UF;
    use UniformSemantic as US;
    use VertexSemantic as VS;

    let p = PipelineDescriptor {
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        vertex_stage: VertexStage {
            shader: "pipeline/hello.vs".to_owned(),
            attributes: [
                PipelineAttribute(0, VS::Position, VF::Float2),
                PipelineAttribute(1, VS::Normal, VF::Float3),
                PipelineAttribute(3, VS::Color(3), VF::Float3),
                PipelineAttribute(2, VS::Custom("c1".to_owned()), VF::Ushort2Norm),
            ]
            .iter()
            .cloned()
            .collect(),
            global_uniforms: [
                //PipelineUniform(0, US::ModelView, UF::Binary),
                PipelineUniform(1, US::Diffuse, UF::Texture),
                PipelineUniform(2, US::Diffuse, UF::Sampler),
                PipelineUniform(3, US::Custom("u1".to_owned()), UF::Texture),
            ]
            .iter()
            .cloned()
            .collect(),
            local_uniforms: [/*PipelineUniform(0, US::ModelView, UF::Binary)*/]
                .iter()
                .cloned()
                .collect(),
        },
        fragment_stage: FragmentStage {
            shader: "pipeline/hello.fs".to_owned(),
            global_uniforms: Default::default(),
            local_uniforms: Default::default(),
        },
        color_stage: Blending::Replace,
        //depth_stencile_stage:
    };

    println!("{}", serde_json::to_string(&p).unwrap());
}
