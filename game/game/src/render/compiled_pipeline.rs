use crate::{
    assets::{
        AssetError, PipelineDescriptor, PipelineUniform, PipelineUniformLayout, ShaderType, Uniform, UniformScope,
        VertexBufferLayout, VertexStage,
    },
    render::Compile,
};

struct PipelineBindGroupLayout {
    layout: wgpu::BindGroupLayout,
    uniforms: Vec<PipelineUniform>,
}

pub struct PipelineBindGroup {
    pub auto: wgpu::BindGroup,
    pub global: wgpu::BindGroup,
    pub local: wgpu::BindGroup,
}

/// Compiled pipeline with related binding information
pub struct CompiledPipeline {
    pub pipeline: wgpu::RenderPipeline,
    auto_bind_group_layout: PipelineBindGroupLayout,
    global_bind_group_layout: PipelineBindGroupLayout,
    local_bind_group_layout: PipelineBindGroupLayout,
}

impl CompiledPipeline {
    fn get_bind_group(&self, scope: UniformScope) -> &PipelineBindGroupLayout {
        match scope {
            UniformScope::Auto => &self.auto_bind_group_layout,
            UniformScope::Global => &self.global_bind_group_layout,
            UniformScope::Local => &self.local_bind_group_layout,
        }
    }

    fn get_uniform_buffer_size(&self, scope: UniformScope) -> usize {
        let bind_group = self.get_bind_group(scope);
        bind_group.uniforms.iter().fold(0, |size, u| {
            if let Uniform::UniformBuffer(b) = u.uniform() {
                size + b.size()
            } else {
                size
            }
        })
    }

    pub fn create_uniform_buffer(&self, scope: UniformScope, device: &wgpu::Device) -> Option<wgpu::Buffer> {
        let size = self.get_uniform_buffer_size(scope);
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
        device: &wgpu::Device,
        scope: UniformScope,
        mut get_value: F,
    ) -> wgpu::BindGroup
    where
        F: FnMut(&Uniform) -> wgpu::BindingResource<'a>,
    {
        let bind_group = self.get_bind_group(scope);
        let mut bindings = Vec::with_capacity(bind_group.uniforms.len());
        for u in bind_group.uniforms.iter() {
            let resource = get_value(u.uniform());
            //todo: check if resource is conforming to uniform
            bindings.push(wgpu::BindGroupEntry {
                binding: u.location(),
                resource,
            });
        }

        log::trace!("create_bind_group for {:?}: {:?}", scope, bind_group.layout);

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group.layout,
            entries: &bindings,
        })
    }

    pub fn create_bind_groups<'a, FA, FG, FL>(
        &self,
        device: &wgpu::Device,
        get_auto: FA,
        get_global: FG,
        get_local: FL,
    ) -> PipelineBindGroup
    where
        FA: FnMut(&Uniform) -> wgpu::BindingResource<'a>,
        FG: FnMut(&Uniform) -> wgpu::BindingResource<'a>,
        FL: FnMut(&Uniform) -> wgpu::BindingResource<'a>,
    {
        PipelineBindGroup {
            auto: self.create_bind_group(device, UniformScope::Auto, get_auto),
            global: self.create_bind_group(device, UniformScope::Global, get_global),
            local: self.create_bind_group(device, UniformScope::Local, get_local),
        }
    }
}

type CompileExtra<'a, F> = (wgpu::TextureFormat, &'a [VertexBufferLayout], F);

impl PipelineDescriptor {
    fn create_attribute_descriptors(
        &self,
        vertex_layouts: &[VertexBufferLayout],
    ) -> Result<Vec<(wgpu::BufferAddress, Vec<wgpu::VertexAttributeDescriptor>)>, AssetError> {
        let mut descriptors = Vec::new();
        for layout in vertex_layouts {
            let mut attributes = Vec::new();
            for va in &layout.attributes {
                if let Some(pa) = self
                    .vertex_stage
                    .attributes
                    .iter()
                    .find(|pa| pa.semantic() == va.semantic())
                {
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
        &self,
        device: &wgpu::Device,
        scope: UniformScope,
    ) -> Result<PipelineBindGroupLayout, AssetError> {
        let uniform_layout = self.get_uniform_layout(scope)?;
        log::trace!("Bind group({:?}) layout {:#?}", scope, uniform_layout);

        let bindings = Self::create_bind_group_layout_entries(&uniform_layout)?;
        let uniforms: Vec<_> = uniform_layout.iter().map(|(u, _)| u).cloned().collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &bindings,
        });

        Ok(PipelineBindGroupLayout { layout, uniforms })
    }
}

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

        let vertex_buffers = self.create_attribute_descriptors(&vertex_layouts)?;
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

        let auto_bind_group_layout = self.create_bind_group_layout(device, UniformScope::Auto)?;
        let global_bind_group_layout = self.create_bind_group_layout(device, UniformScope::Global)?;
        let local_bind_group_layout = self.create_bind_group_layout(device, UniformScope::Local)?;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &auto_bind_group_layout.layout,
                &global_bind_group_layout.layout,
                &local_bind_group_layout.layout,
            ],
            push_constant_ranges: &[],
        });

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

        Ok(CompiledPipeline {
            pipeline,
            auto_bind_group_layout,
            global_bind_group_layout,
            local_bind_group_layout,
        })
    }
}
