use crate::resources::{Resource, ResourceBakeContext, ResourceId};

pub trait ResourceConfig<T: Resource> {
    fn auto_build(&self) -> bool;

    fn build(&self, id: &ResourceId) -> T;

    fn post_process(&self, context: &mut ResourceBakeContext<'_, T>);

    fn auto_gc(&self) -> bool;
}

/// Resources are added and removed manually
#[derive(Default)]
pub struct UnmanagedResource;

impl<T: Resource> ResourceConfig<T> for UnmanagedResource {
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
/// functor.
#[derive(Default)]
pub struct ManagedResource<T, F>
where
    T: Resource,
    F: Fn(&ResourceId) -> T,
{
    build: F,
}

impl<T, F> ManagedResource<T, F>
where
    T: Resource,
    F: Fn(&ResourceId) -> T,
{
    pub fn new(build: F) -> Self {
        Self { build }
    }
}

impl<T, F> ResourceConfig<T> for ManagedResource<T, F>
where
    T: Resource,
    F: Fn(&ResourceId) -> T,
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
