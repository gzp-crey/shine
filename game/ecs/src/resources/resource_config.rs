use crate::resources::{Resource, ResourceBakeContext, ResourceId};

/// Resource store configuration.
pub struct ResourceConfiguration<T: Resource> {
    /// Optional functor to create missing resources from id
    pub build: Option<Box<dyn Fn(&ResourceId) -> T>>,

    /// Optional functor to call during bake
    pub post_process: Option<Box<dyn Fn(&mut ResourceBakeContext<'_, T>)>>,

    /// Remove unreferenced resources during maintain
    pub auto_gc: bool,
    // General E to add extra functionality to the resource management
    //extension: E,
}

impl<T: Resource> Default for ResourceConfiguration<T> {
    fn default() -> Self {
        Self {
            build: None,
            post_process: None,
            auto_gc: false,
        }
    }
}

impl<T: Resource> ResourceConfiguration<T> {
    /*pub fn extension(&self) -> E {
        &self.extension()
    }*/
    //fn with_build()
}
