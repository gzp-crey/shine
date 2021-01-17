use crate::{
    assets::{PipelineStateDescriptor, RenderTargetDescriptor},
    render::{FrameTarget, FrameTargetResMut, TextureTarget, TextureTargetsResMut},
};
use shine_ecs::ecs::resources::{
    FetchResource, ResourceQuery, ResourceClaim, ResourceClaimScope, ResourceClaims, ResourceAccess, ResourceTag,
    Resources, TagMut,
};

#[derive(Clone, Debug, PartialEq)]
struct TargetIndex {
    generation: usize,
    index: usize,
}

impl TargetIndex {
    fn frame_target() -> Self {
        Self {
            generation: 0,
            index: usize::max_value(),
        }
    }

    fn texture_target(generation: usize, index: usize) -> Self {
        Self { generation, index }
    }

    fn is_frame_target(&self) -> bool {
        self.index == usize::max_value()
    }

    fn is_texture_target(&self) -> bool {
        !self.is_frame_target()
    }
}

struct Inner {
    color_target_indices: Vec<TargetIndex>,
    depth_target_index: Option<TargetIndex>,
    pipeline_state: PipelineStateDescriptor,
}

#[derive(Default)]
pub struct RenderTarget {
    descriptor: RenderTargetDescriptor,
    inner: Option<Inner>,
}

impl RenderTarget {
    fn new(tag: ResourceTag, descriptor: RenderTargetDescriptor) -> (RenderTarget, RenderTargetClaim) {
        let claim = RenderTargetClaim::from_descriptor(tag, descriptor);
        let target = Self {
            descriptor,
            inner: None,
        };
        (target, claim)
    }

    fn is_dirty(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        _frame_target: &FrameTargetResMut,
        texture_targets: &TextureTargetsResMut,
    ) -> bool {
        if let Some(inner) = &mut self.inner {
            if let Some(depth_target) = &mut inner.depth_target_index {
                if texture_targets[depth_target.index].generation() != depth_target.generation {
                    log::debug!(
                        "TextureTarget {:?} changed, RenderTarget re-compile required (depth)",
                        descriptor.depth.as_ref().map(|depth| &depth.target),
                    );
                    return true;
                }
            }

            for (color_idx, color_target) in inner.color_target_indices.iter_mut().enumerate() {
                if color_target.is_texture_target() {
                    // texture target
                    if texture_targets[color_target.index].generation() != color_target.generation {
                        log::debug!(
                            "TextureTarget {:?} changed, RenderTarget re-compile required (color({}))",
                            &descriptor.colors[color_idx].target,
                            color_idx
                        );
                        return true;
                    }
                } else {
                    // frame target
                    // noop, fixed format won't change
                }
            }
            false
        } else {
            true
        }
    }

    pub fn release(&mut self) {
        self.inner = None;
    }

    pub fn resolve(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        frame_target: &FrameTargetResMut,
        texture_targets: &TextureTargetsResMut,
    ) {
        if !self.is_dirty(descriptor, frame_target, texture_targets) {
            return;
        }

        // resolve depth
        let (depth_state, depth_target_index) = if let Some(depth) = &descriptor.depth {
            let target_index = texture_targets.position_by_name(&depth.target).unwrap();
            let texture_target = &texture_targets[target_index];
            let target_index = TargetIndex::texture_target(texture_target.generation(), target_index);
            let state = wgpu::DepthStencilStateDescriptor {
                format: texture_target.descriptor().format,
                depth_write_enabled: depth.depth_operation.write_enabled,
                depth_compare: depth.depth_operation.compare,
                stencil: wgpu::StencilStateDescriptor {
                    front: depth.stencil_operation.front.clone(),
                    back: depth.stencil_operation.back.clone(),
                    read_mask: depth.stencil_operation.read_mask,
                    write_mask: depth.stencil_operation.write_mask,
                },
            };
            (Some(state), Some(target_index))
        } else {
            (None, None)
        };

        // resolve colors
        let mut color_target_indices = Vec::with_capacity(descriptor.colors.len());
        let mut color_states = Vec::with_capacity(descriptor.colors.len());
        for color in descriptor.colors.iter() {
            let (state, target_index) = if let Some(color_target) = &color.target {
                // texture target
                let target_index = texture_targets.position_by_name(color_target).unwrap();
                let texture_target = &texture_targets[target_index];
                let target_index = TargetIndex::texture_target(texture_target.generation(), target_index);
                let state = wgpu::ColorStateDescriptor {
                    format: texture_target.descriptor().format,
                    alpha_blend: color.alpha_blend.clone(),
                    color_blend: color.color_blend.clone(),
                    write_mask: color.write_mask,
                };
                (state, target_index)
            } else {
                // frame target
                let target_index = TargetIndex::frame_target();
                let state = wgpu::ColorStateDescriptor {
                    format: frame_target.descriptor().unwrap().format,
                    alpha_blend: color.alpha_blend.clone(),
                    color_blend: color.color_blend.clone(),
                    write_mask: color.write_mask,
                };
                (state, target_index)
            };
            color_target_indices.push(target_index);
            color_states.push(state);
        }

        self.inner = Some(Inner {
            color_target_indices,
            depth_target_index,
            pipeline_state: PipelineStateDescriptor {
                depth_state,
                color_states,
            },
        });
    }
}

struct ClaimInner {
    tag: ResourceTag,
    depth_texture: Option<ResourceTag>,
    color_textures: Vec<ResourceTag>,
}

#[derive(Default, Debug)]
pub struct RenderTargetClaim {
    inner: Option<ClaimInner>,
}

impl RenderTargetClaim {
    fn from_descriptor(tag: ResourceTag, descriptor: RenderTargetDescriptor) -> Self {
        let inner = ClaimInner {
            tag,
            depth_texture: descriptor.depth.map(|depth| depth.texture.clone()),
            color_textures: descriptor.colors.iter().map(|color| color.texture.clone()).collect(),
        };
        Self { inner: Some(inner) }
    }
}

impl ResourceQuery for RenderTargetClaim {
    fn into_claim(&self) -> ResourceClaim {
        //let render_target = Some(ResourceIndex::new::<RenderTarget>(Some(render_target_name)));
        //let color_targets = descriptor.map(|descriptor| descriptor.colors.target)
        //let depth_targets = descriptor.map(|descriptor| descriptor.depth.target)
        ResourceClaim::none()
    }
}

/*
/// Unique borrow of a render target
pub struct RenderTargetRes<'a> {
    render_target: NamedResMut<'a, RenderTarget>,
    frame_target: FrameTargetResMut<'a>,
    texture_targets: TextureTargetsResMut<'a>,
}

impl<'a> ResourceAccess for RenderTargetRes<'a> {
    type Fetch = FetchRenderTarget;
    type Claim = Option<RenderTargetClaim>;

    fn default_claim()  -> ResourceClaim {
        ResourceClaim::new(None, Some(ResourceIndex::new::<FrameTarget>(None))),
    }
}


fn add_extra_claim(claim: Self::Claim, resource_claims: &mut ResourceClaims) {
    let (render_target_name, texture_target_names) = claim;
    let render_target = Some(ResourceIndex::new::<RenderTarget>(Some(render_target_name)));
    let texture_targets = texture_target_names
        .into_iter()
        .map(|name| ResourceIndex::new::<TextureTarget>(Some(name)));
    let claims = render_target.into_iter().chain(texture_targets);
    resource_claims.add_claim::<Self::Fetch>(ResourceClaimScope::Extra, ResourceClaim::new(None, claims));
}

pub struct FetchRenderTarget;

impl<'a> FetchResource<'a> for FetchRenderTarget {
    type Item = RenderTargetRes<'a>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        let claims = resource_claims.get_claims::<Self>(ResourceClaimScope::Extra).unwrap();
        let mut claims = claims.mutable.iter().map(|x| x.name().unwrap().to_owned());
        let render_target_name = claims.next().unwrap();
        let render_target = NamedResMut(resources.get_mut_with_name::<T>(render_target_name).unwrap());
        let texture_target_names = claims.mutable.iter().map(|x| x.name().unwrap().to_owned());
        let texture_targets = NamedResMut(resources.get_mut_with_names::<T, _>(texture_target_names).unwrap());
        let frame_target = ResMut(resources.get_mut::<FrameTarget>().unwrap());

        RenderTargetRes {
            render_target,
            frame_target,
            texture_targets,
        }
    }
}
*/
