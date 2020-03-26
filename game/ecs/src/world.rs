use crate::resources::{named, unnamed};
use shred::{self, Fetch, FetchMut};

pub use shred::World;

/// Store
///  - mapping from a uniqe id to data
///  - allow creating handles on demand without blocking, but actual loading is deffered
///  - mainly used to store resources shared by multiple entity (ex textures, geometry, etc.)
///  - reading and update stores are exclusive and update is performed in a blocking pass
pub trait StoreWorld {
    fn register_named_store<D: 'static + named::Data>(&mut self);
    fn named_store<D: 'static + named::Data>(&self) -> Fetch<'_, named::Store<D>>;
    fn named_store_mut<D: 'static + named::Data>(&self) -> FetchMut<'_, named::Store<D>>;

    fn register_store<D: 'static>(&mut self);
    fn store<D: 'static>(&self) -> Fetch<'_, unnamed::Store<D>>;
    fn store_mut<D: 'static>(&self) -> FetchMut<'_, unnamed::Store<D>>;
}

/// General resource management for custom user resources
pub trait ResourceWorld {
    fn register_resource<D: 'static + Send + Sync + Default>(&mut self);
    fn register_resource_with<D: 'static + Send + Sync>(&mut self, resource: D);
    fn resource<D: 'static + Send + Sync>(&self) -> Fetch<'_, D>;
    fn resource_mut<D: 'static + Send + Sync>(&self) -> FetchMut<'_, D>;
}

impl StoreWorld for World {
    fn register_named_store<D: 'static + named::Data>(&mut self) {
        self.insert::<named::Store<D>>(Default::default());
    }

    fn named_store<D: 'static + named::Data>(&self) -> Fetch<'_, named::Store<D>> {
        self.fetch()
    }

    fn named_store_mut<D: 'static + named::Data>(&self) -> FetchMut<'_, named::Store<D>> {
        self.fetch_mut()
    }

    fn register_store<D: 'static>(&mut self) {
        self.insert::<unnamed::Store<D>>(Default::default());
    }

    fn store<D: 'static>(&self) -> Fetch<'_, unnamed::Store<D>> {
        self.fetch()
    }

    fn store_mut<D: 'static>(&self) -> FetchMut<'_, unnamed::Store<D>> {
        self.fetch_mut()
    }
}

impl ResourceWorld for World {
    fn register_resource<D: 'static + Send + Sync + Default>(&mut self) {
        self.insert::<D>(Default::default());
    }

    fn register_resource_with<D: 'static + Send + Sync>(&mut self, resource: D) {
        self.insert::<D>(resource);
    }

    fn resource<D: 'static + Send + Sync>(&self) -> Fetch<'_, D> {
        self.fetch()
    }

    fn resource_mut<D: 'static + Send + Sync>(&self) -> FetchMut<'_, D> {
        self.fetch_mut()
    }
}
