use crate::render::{Context, VertexBufferLayout};
use crate::wgpu;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Hash, Deserialize, PartialEq, Eq)]
pub enum VertexSemantic {
    Position,
    Color(u8),
    Texture(u8),
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Uniform {
    ModelView,
    //Named(String, wgpu::VertexFormat),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: Vec<PipelineAttribute>,
    pub global_uniforms: HashMap<u8, Uniform>,
    pub local_uniforms: HashMap<u8, Uniform>,
}

impl VertexStage {
    fn check_format(&self, target: wgpu::VertexFormat, source: wgpu::VertexFormat) -> Result<(), String> {
        if target != source {
            Err(format!(
                "Vertex attribute format missmacth, target:{:?}, source:{:?}",
                target, source
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_vertex_layouts(&self, vertex_layouts: &Vec<VertexBufferLayout>) -> Result<(), String> {
        // check vertex attribute source duplication and the format compatibility
        let mut semantics = HashSet::new();
        for layout in vertex_layouts {
            for va in &layout.attributes {
                if let Some(pa) = self.attributes.iter().find(|pa| pa.semantic() == va.semantic()) {
                    self.check_format(pa.format(), va.format())?;
                    if !semantics.insert(pa.semantic().clone()) {
                        return Err(format!("{:?} attribute defined multiple times", pa.semantic()));
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
                return Err(format!("Missing attribute for {:?}", pa.semantic()));
            }
        }

        Ok(())
    }

    pub fn create_attribute_descriptors(
        &self,
        vertex_layouts: &Vec<VertexBufferLayout>,
    ) -> Result<Vec<(wgpu::BufferAddress, Vec<wgpu::VertexAttributeDescriptor>)>, String> {
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
    pub global_uniforms: HashMap<u8, Uniform>,
    pub local_uniforms: HashMap<u8, Uniform>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Blending {
    Replace,
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
    pub fn compile(
        &self,
        context: &Context,
        vertex_layouts: &Vec<VertexBufferLayout>,
        (vs, fs): (&wgpu::ShaderModule, &wgpu::ShaderModule),
    ) -> Result<wgpu::RenderPipeline, String> {
        let device = context.device();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[],
        });

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
                format: context.swap_chain_format(),
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

        Ok(pipeline)
    }
}

pub fn foo() {
    use Uniform::*;
    use VertexSemantic::*;

    let p = PipelineDescriptor {
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        vertex_stage: VertexStage {
            shader: "pipeline/hello.vs".to_owned(),
            attributes: [
                PipelineAttribute(0, Position, wgpu::VertexFormat::Float2),
                PipelineAttribute(1, Normal, wgpu::VertexFormat::Float3),
                PipelineAttribute(3, Color(3), wgpu::VertexFormat::Float3),
                PipelineAttribute(2, Custom("c1".to_owned()), wgpu::VertexFormat::Ushort2Norm),
            ]
            .iter()
            .cloned()
            .collect(),
            global_uniforms: [(0, ModelView)].iter().cloned().collect(),
            local_uniforms: [(0, ModelView)].iter().cloned().collect(),
        },
        fragment_stage: FragmentStage {
            shader: "pipeline/hello.fs".to_owned(),
            global_uniforms: [(0, ModelView)].iter().cloned().collect(),
            local_uniforms: Default::default(),
        },
        color_stage: Blending::Replace,
        //depth_stencile_stage:
    };

    println!("{}", serde_json::to_string(&p).unwrap());
}
