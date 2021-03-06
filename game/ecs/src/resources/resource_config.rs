use crate::resources::{Resource, ResourceBakeContext, ResourceHandle, ResourceId};
use std::{any::Any, marker::PhantomData};

pub trait ResourceConfig {
    type Resource: Resource + Sized;

    fn as_any(&self) -> &dyn Any;

    /// Indicates if build should be called when a resoucre was not found.
    fn auto_build(&self) -> bool;

    /// Called to create missing resources when auto_build is enabled.
    fn build(&self, handle: ResourceHandle<Self::Resource>, id: &ResourceId) -> Self::Resource;

    /// Called during bake to perform additional updates on the resources (ex. consume async load responses)
    fn post_bake(&mut self, context: &mut ResourceBakeContext<'_, Self::Resource>);
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
}

/// Resources are added and removed automatically using the provided builder
/// functors.
pub struct ManagedResource<T: Resource> {
    build: Box<dyn Fn(&ResourceId) -> T>,
}

impl<T: Resource> ManagedResource<T> {
    pub fn new<F: 'static + Fn(&ResourceId) -> T>(build: F) -> Self {
        Self { build: Box::new(build) }
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
}
