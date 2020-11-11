use crate::resources::{Resource, ResourceBakeContext, ResourceId};
use std::marker::PhantomData;

pub trait ResourceConfig: Sized {
    type Resource: Resource<Config = Self>;

    fn auto_build(&self) -> bool;

    fn build(&self, id: &ResourceId) -> Self::Resource;

    fn post_process(&self, context: &mut ResourceBakeContext<'_, Self::Resource>);

    fn auto_gc(&self) -> bool;
}

/// Resources configuration to manage the resource manually.
/// The resources have to be added or removed explicitly and no automatic creation or
/// release happens.
pub struct UnmanagedResource<T>
where
    T: Resource<Config = Self>,
{
    _ph: PhantomData<T>,
}

impl<T> Default for UnmanagedResource<T>
where
    T: Resource<Config = Self>,
{
    fn default() -> Self {
        Self { _ph: PhantomData }
    }
}

impl<T> ResourceConfig for UnmanagedResource<T>
where
    T: Resource<Config = Self>,
{
    type Resource = T;

    fn auto_build(&self) -> bool {
        false
    }

    fn build(&self, _id: &ResourceId) -> Self::Resource {
        unreachable!()
    }

    fn post_process(&self, _context: &mut ResourceBakeContext<'_, Self::Resource>) {}

    fn auto_gc(&self) -> bool {
        false
    }
}

/// Resources are added and removed automatically using the provided builder
/// functors.
pub struct ManagedResource<T>
where
    T: Resource<Config = Self>,
{
    build: Box<dyn Fn(&ResourceId) -> T>,
    auto_gc: bool,
}

impl<T> ManagedResource<T>
where
    T: Resource<Config = Self>,
{
    pub fn new<F: 'static + Fn(&ResourceId) -> T>(auto_gc: bool, build: F) -> Self {
        Self {
            build: Box::new(build),
            auto_gc,
        }
    }
}

impl<T> ResourceConfig for ManagedResource<T>
where
    T: Resource<Config = Self>,
{
    type Resource = T;

    fn auto_build(&self) -> bool {
        true
    }

    fn build(&self, id: &ResourceId) -> Self::Resource {
        (self.build)(id)
    }

    fn post_process(&self, _context: &mut ResourceBakeContext<'_, Self::Resource>) {}

    fn auto_gc(&self) -> bool {
        self.auto_gc
    }
}
