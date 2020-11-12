use crate::resources::{Resource, ResourceBakeContext, ResourceHandle, ResourceId};
use std::{any::Any, marker::PhantomData};

pub trait ResourceConfig {
    type Resource: Resource + Sized;

    fn as_any(&self) -> &dyn Any;

    fn auto_build(&self) -> bool;

    fn build(&self, handle: ResourceHandle<Self::Resource>, id: &ResourceId) -> Self::Resource;

    fn post_bake(&mut self, context: &mut ResourceBakeContext<'_, Self::Resource>);

    fn auto_gc(&self) -> bool;
}

/// Resources configuration to manage the resource manually.
/// The resources have to be added or removed explicitly and no automatic creation or
/// release happens.
pub struct UnmanagedResource<T: Resource> {
    _ph: PhantomData<T>,
}

impl<T: Resource> Default for UnmanagedResource<T> {
    fn default() -> Self {
        Self { _ph: PhantomData }
    }
}

impl<T: Resource> ResourceConfig for UnmanagedResource<T> {
    type Resource = T;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn auto_build(&self) -> bool {
        false
    }

    fn build(&self, _handle: ResourceHandle<Self::Resource>, _id: &ResourceId) -> Self::Resource {
        unreachable!()
    }

    fn post_bake(&mut self, _context: &mut ResourceBakeContext<'_, Self::Resource>) {}

    fn auto_gc(&self) -> bool {
        false
    }
}

/// Resources are added and removed automatically using the provided builder
/// functors.
pub struct ManagedResource<T: Resource> {
    build: Box<dyn Fn(&ResourceId) -> T>,
    auto_gc: bool,
}

impl<T: Resource> ManagedResource<T> {
    pub fn new<F: 'static + Fn(&ResourceId) -> T>(auto_gc: bool, build: F) -> Self {
        Self {
            build: Box::new(build),
            auto_gc,
        }
    }
}

impl<T: Resource> ResourceConfig for ManagedResource<T> {
    type Resource = T;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn auto_build(&self) -> bool {
        true
    }

    fn build(&self, _handle: ResourceHandle<Self::Resource>, id: &ResourceId) -> Self::Resource {
        (self.build)(id)
    }

    fn post_bake(&mut self, _context: &mut ResourceBakeContext<'_, Self::Resource>) {}

    fn auto_gc(&self) -> bool {
        self.auto_gc
    }
}
