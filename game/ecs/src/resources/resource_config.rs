use crate::resources::{Resource, ResourceBakeContext, ResourceId};

pub trait ResourceConfig<T>: Sized
where
    T: Resource<Config = Self>,
{
    fn auto_build(&self) -> bool;

    fn build(&self, id: &ResourceId) -> T;

    fn post_process(&self, context: &mut ResourceBakeContext<'_, T>);

    fn auto_gc(&self) -> bool;
}

/// Resources are added and removed manually
#[derive(Default)]
pub struct UnmanagedResource;

impl<T> ResourceConfig<T> for UnmanagedResource
where
    T: Resource<Config = Self>,
{
    fn auto_build(&self) -> bool {
        false
    }

    fn build(&self, _id: &ResourceId) -> T {
        unreachable!()
    }

    fn post_process(&self, _context: &mut ResourceBakeContext<'_, T>) {}

    fn auto_gc(&self) -> bool {
        false
    }
}

/// Resources are added and removed automatically using the provided builder
/// functor. If there is no active handlue during bake, the unreferenced resources are
/// removed.
pub struct ManagedResource<T>
where
    T: Resource<Config = Self>,
{
    build: Box<dyn Fn(&ResourceId) -> T>,
}

impl<T> ManagedResource<T>
where
    T: Resource<Config = Self>,
{
    pub fn new<F: 'static + Fn(&ResourceId) -> T>(build: F) -> Self {
        Self { build: Box::new(build) }
    }
}

impl<T> ResourceConfig<T> for ManagedResource<T>
where
    T: Resource<Config = Self>,
{
    fn auto_build(&self) -> bool {
        true
    }

    fn build(&self, id: &ResourceId) -> T {
        (self.build)(id)
    }

    fn post_process(&self, _context: &mut ResourceBakeContext<'_, T>) {}

    fn auto_gc(&self) -> bool {
        true
    }
}
