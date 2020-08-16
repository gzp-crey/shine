use crate::{
    assets::{
        AssetError, PipelineDescriptor, PipelineUniform, PipelineUniformLayout, ShaderType, Uniform,
        VertexBufferLayout, VertexStage, MAX_UNIFORM_GROUP_COUNT,
    },
    render::Compile,
};

/// Compiled pipeline with related binding information
pub struct CompiledPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub uniforms: [Option<(wgpu::BindGroupLayout, Vec<PipelineUniform>)>; MAX_UNIFORM_GROUP_COUNT],
}

impl CompiledPipeline {
    fn get_uniform_buffer_size(&self, group: u32) -> usize {
        self.uniforms[group as usize]
            .as_ref()
            .map(|layout| {
                layout.1.iter().fold(0, |size, u| {
                    if let Uniform::UniformBuffer(b) = u.uniform() {
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
                mapped_at_creation: false,
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
                bindings.push(wgpu::BindGroupEntry {
                    binding: u.location(),
                    resource,
                });
            }

            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: bind_group_layout,
                entries: &bindings,
            }))
        } else {
            None
        }
    }

    pub fn render<'a, 'b, F>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'a, 'b>,
        render: F,
    ) where
        F: FnOnce(&mut wgpu::RenderPass),
    {
        let mut pass = encoder.begin_render_pass(pass_descriptor);
        pass.set_pipeline(&self.pipeline);
        render(&mut pass);
    }
}

fn create_bind_group_layout_entries(
    layout: &PipelineUniformLayout,
) -> Result<Vec<wgpu::BindGroupLayoutEntry>, AssetError> {
    let mut descriptor = Vec::new();
    for (uniform, stages) in layout.iter() {
        descriptor.push(match uniform.uniform() {
            Uniform::Texture(_) => wgpu::BindGroupLayoutEntry {
                binding: uniform.location(),
                visibility: *stages,
                ty: wgpu::BindingType::SampledTexture {
                    multisampled: false,
                    dimension: wgpu::TextureViewDimension::D2,
                    component_type: wgpu::TextureComponentType::Float,
                },
                count: None,
            },
            Uniform::Sampler(_) => wgpu::BindGroupLayoutEntry {
                binding: uniform.location(),
                visibility: *stages,
                ty: wgpu::BindingType::Sampler { comparison: false },
                count: None,
            },
            Uniform::UniformBuffer(sem) => wgpu::BindGroupLayoutEntry {
                binding: uniform.location(),
                visibility: *stages,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: wgpu::BufferSize::new(sem.size() as u64),
                },
                count: None,
            },
        });
    }
    Ok(descriptor)
}

fn create_bind_group_layout(
    layout: &PipelineUniformLayout,
    device: &wgpu::Device,
) -> Result<Option<(wgpu::BindGroupLayout, Vec<PipelineUniform>)>, AssetError> {
    let bindings = create_bind_group_layout_entries(layout)?;
    let uniforms: Vec<_> = layout.iter().map(|(u, _)| u).cloned().collect();
    if !bindings.is_empty() {
        Ok(Some((
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &bindings,
            }),
            uniforms,
        )))
    } else {
        Ok(None)
    }
}

fn create_attribute_descriptors(
    vertex_stage: &VertexStage,
    vertex_layouts: &[VertexBufferLayout],
) -> Result<Vec<(wgpu::BufferAddress, Vec<wgpu::VertexAttributeDescriptor>)>, AssetError> {
    let mut descriptors = Vec::new();
    for layout in vertex_layouts {
        let mut attributes = Vec::new();
        for va in &layout.attributes {
            if let Some(pa) = vertex_stage.attributes.iter().find(|pa| pa.semantic() == va.semantic()) {
                VertexStage::check_vertex_format(pa.format(), va.format())?;

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

type CompileExtra<'a, F> = (wgpu::TextureFormat, &'a [VertexBufferLayout], F);

impl<'a, F> Compile<CompileExtra<'a, F>> for PipelineDescriptor
where
    F: FnMut(ShaderType) -> Result<&'a wgpu::ShaderModule, AssetError>,
{
    type Compiled = Result<CompiledPipeline, AssetError>;

    fn compile(
        &self,
        device: &wgpu::Device,
        (color_state_format, vertex_layouts, mut get_shader): CompileExtra<'a, F>,
    ) -> Self::Compiled {
        self.vertex_stage.check_vertex_layouts(&vertex_layouts)?;

        let vertex_buffers = create_attribute_descriptors(&self.vertex_stage, &vertex_layouts)?;
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
        log::trace!("Vertex state: {:#?}", vertex_state);

        let mut uniforms = [None, None, None];
        if uniforms.len() != MAX_UNIFORM_GROUP_COUNT {
            return Err(AssetError::Content(format!(
                "Invalid uniform group count: {}, expected: {}",
                uniforms.len(),
                MAX_UNIFORM_GROUP_COUNT,
            )));
        }
        for (i, uniform) in uniforms.iter_mut().enumerate() {
            let layout = self.get_uniform_layout(i as u32)?;
            log::trace!("Bind group({}) layout {:#?}", i, layout);
            *uniform = create_bind_group_layout(&layout, device)?;
        }

        let pipeline_layout = {
            let mut bind_group_layouts = Vec::new();
            for uniform in uniforms.iter() {
                if let Some((ref bind_group_layout, _)) = uniform {
                    bind_group_layouts.push(bind_group_layout);
                }
            }
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &bind_group_layouts,
                push_constant_ranges: &[],
            })
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            primitive_topology: self.primitive_topology,

            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: get_shader(ShaderType::Vertex)?,
                entry_point: "main",
            },

            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: get_shader(ShaderType::Fragment)?,
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

        Ok(CompiledPipeline { pipeline, uniforms })
    }
}
