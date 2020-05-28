use crate::assets::{AssetError, Uniform, VertexBufferLayout, VertexSemantic};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

pub const UNIFORM_GROUP_COUNT: u32 = 2;
pub const GLOBAL_UNIFORMS: u32 = 0;
pub const LOCAL_UNIFORMS: u32 = 1;

/// A vertex attribute requirement of the pipeline
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

/// A uniform requirement of the pipeline
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

/// Combined uniform requirements of the pipeline
#[derive(Debug)]
pub struct PipelineUniformLayout(Vec<(PipelineUniform, wgpu::ShaderStage)>);

impl PipelineUniformLayout {
    pub fn from_stages(layouts: &[(&[PipelineUniform], wgpu::ShaderStage)]) -> Result<Self, AssetError> {
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

    pub fn create_bind_group_layout_entries(&self) -> Result<Vec<wgpu::BindGroupLayoutEntry>, AssetError> {
        let mut descriptor = Vec::new();
        for (uniform, stages) in self.0.iter() {
            descriptor.push(match uniform.1 {
                Uniform::Texture(_) => wgpu::BindGroupLayoutEntry {
                    binding: uniform.location(),
                    visibility: *stages,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                    },
                },
                Uniform::Sampler(_) => wgpu::BindGroupLayoutEntry {
                    binding: uniform.location(),
                    visibility: *stages,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                },
                Uniform::UniformBuffer(_) => wgpu::BindGroupLayoutEntry {
                    binding: uniform.location(),
                    visibility: *stages,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
            });
        }
        Ok(descriptor)
    }

    pub fn create_bind_group_layout(
        self,
        device: &wgpu::Device,
    ) -> Result<Option<(wgpu::BindGroupLayout, Vec<PipelineUniform>)>, AssetError> {
        let bindings = self.create_bind_group_layout_entries()?;
        let layout: Vec<_> = self.0.into_iter().map(|(u, _)| u).collect();
        if !bindings.is_empty() {
            Ok(Some((
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    bindings: &bindings,
                }),
                layout,
            )))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: Vec<PipelineAttribute>,
    pub uniforms: [Vec<PipelineUniform>; UNIFORM_GROUP_COUNT as usize],
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
    pub uniforms: [Vec<PipelineUniform>; UNIFORM_GROUP_COUNT as usize],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Blending {
    Replace,
}

/// Compiled pipeline with related binding information
pub struct PipelineBuffer {
    pub pipeline: wgpu::RenderPipeline,
    pub uniforms: [Option<(wgpu::BindGroupLayout, Vec<PipelineUniform>)>; UNIFORM_GROUP_COUNT as usize],
}

impl PipelineBuffer {
    fn get_uniform_buffer_size(&self, group: u32) -> usize {
        self.uniforms[group as usize]
            .as_ref()
            .map(|layout| {
                layout.1.iter().fold(0, |size, u| {
                    if let Uniform::UniformBuffer(ref b) = u.1 {
                        size + b.size()
                    } else {
                        size
                    }
                })
            })
            .unwrap_or(0)
    }

    pub fn create_buffer(&self, group: u32, device: &wgpu::Device) -> Option<wgpu::Buffer> {
        let size = self.get_uniform_buffer_size(group);
        if size > 0 {
            Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: size as wgpu::BufferAddress,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }))
        } else {
            None
        }
    }

    pub fn create_bind_group<'a, F>(
        &self,
        group: u32,
        device: &wgpu::Device,
        mut get_value: F,
    ) -> Option<wgpu::BindGroup>
    where
        F: FnMut(&Uniform) -> wgpu::BindingResource<'a>,
    {
        if let Some((ref bind_group_layout, ref uniforms)) = self.uniforms[group as usize] {
            let mut bindings = Vec::with_capacity(uniforms.len());
            for u in uniforms {
                let resource = get_value(u.uniform());
                //todo: check if resource is conforming to uniform
                bindings.push(wgpu::Binding {
                    binding: u.location(),
                    resource,
                });
            }

            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: bind_group_layout,
                bindings: &bindings,
            }))
        } else {
            None
        }
    }

    pub fn bind<'a: 'pass, 'pass>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'pass, 'pass>,
    ) -> BoundPipeline<'a, 'pass> {
        let mut b = BoundPipeline {
            pipeline: self,
            render_pass: encoder.begin_render_pass(pass_descriptor),
        };
        b.bind_pipeline();
        b
    }
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
    pub fn get_uniform_layout(&self, group: u32) -> Result<PipelineUniformLayout, AssetError> {
        PipelineUniformLayout::from_stages(&[
            (&self.vertex_stage.uniforms[group as usize], wgpu::ShaderStage::VERTEX),
            (
                &self.fragment_stage.uniforms[group as usize],
                wgpu::ShaderStage::FRAGMENT,
            ),
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
        log::trace!("Vertex state: {:?}", vertex_state);

        let mut uniforms = [None, None];
        for i in 0..UNIFORM_GROUP_COUNT {
            let layout = self.get_uniform_layout(i)?;
            log::trace!("Bind group({}) layout {:#?}", i, layout);
            uniforms[i as usize] = layout.create_bind_group_layout(device)?;
        }

        let pipeline_layout = {
            let mut bind_group_layouts = Vec::new();
            for i in 0..UNIFORM_GROUP_COUNT {
                if let Some((ref bind_group_layout, _)) = uniforms[i as usize] {
                    bind_group_layouts.push(bind_group_layout);
                }
            }
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &bind_group_layouts,
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

        Ok(PipelineBuffer { pipeline, uniforms })
    }
}

/// Pipeline rendering context (render pass)
pub struct BoundPipeline<'a: 'pass, 'pass> {
    pub(crate) pipeline: &'a PipelineBuffer,
    pub(crate) render_pass: wgpu::RenderPass<'pass>,
}

impl<'a: 'pass, 'pass> BoundPipeline<'a, 'pass> {
    #[inline]
    pub(crate) fn bind_pipeline(&mut self) {
        self.render_pass.set_pipeline(&self.pipeline.pipeline);
    }
}

impl<'a: 'pass, 'pass> Deref for BoundPipeline<'a, 'pass> {
    type Target = wgpu::RenderPass<'pass>;
    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}

impl<'a: 'pass, 'pass> DerefMut for BoundPipeline<'a, 'pass> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_pass
    }
}
