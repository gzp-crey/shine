//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::resources::ResourceStoreCell;
use crate::{
    resources::{
        Resource, ResourceHandle, ResourceId, ResourceMultiRead, ResourceMultiWrite, ResourceRead, ResourceStoreRead,
        ResourceStoreWrite, ResourceWrite,
    },
    ECSError,
};
use downcast_rs::{impl_downcast, Downcast};
use std::{
    any::{self, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

/// Helper trait to help implementing downcast for RespurceStore
trait GeneralResourceStoreCell: Downcast {}
impl<T: Resource> GeneralResourceStoreCell for ResourceStoreCell<T> {}
impl_downcast!(GeneralResourceStoreCell);

/// Store all the resources. Unsafe as the Send and Sync property of a resource is not
/// respected.
#[derive(Default)]
struct UnsafeResources {
    map: HashMap<TypeId, Box<dyn GeneralResourceStoreCell>>,
}

/*unsafe impl Send for UnsafeResources {}
unsafe impl Sync for UnsafeResources {}*/

impl UnsafeResources {
    /// # Safety
    /// Resources which are `!Send` must be retrieved or created only on the thread owning the resource
    unsafe fn create_managed<T: Resource>(&mut self, build: Option<Box<dyn Fn(&ResourceId) -> T>>) {
        let ty = TypeId::of::<T>();
        // Managed store have to be registered using the insert_managed
        // function before instances of the resource can be added
        assert!(self.map.get(&ty).is_none());
        self.map.insert(ty, Box::new(ResourceStoreCell::new(build)));
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or created only on the thread owning the resource
    unsafe fn insert<T: Resource>(&mut self, id: ResourceId, resource: T) {
        let ty = TypeId::of::<T>();
        let cell = self
            .map
            .entry(ty)
            .or_insert_with(|| Box::new(ResourceStoreCell::<T>::new(None)))
            .downcast_mut::<ResourceStoreCell<T>>()
            .expect("Downcast error");
        ResourceStoreWrite::new(cell).insert(id, resource);
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or created only on the thread owning the resource
    unsafe fn remove<T: Resource>(&mut self, id: &ResourceId) -> Option<T> {
        self.write_store()?.remove(id)
    }

    /// # Safety
    /// Resources which are `!Sync` must be accessed only on the thread owning the resource
    unsafe fn read_store<T: Resource>(&self) -> Option<ResourceStoreRead<'_, T>> {
        let ty = TypeId::of::<T>();
        let cell = self
            .map
            .get(&ty)?
            .downcast_ref::<ResourceStoreCell<T>>()
            .expect("Downcast error");
        Some(ResourceStoreRead::new(cell))
    }

    /// # Safety
    /// Resources which are `!Send` must be accessed only on the thread owning the resource
    unsafe fn write_store<T: Resource>(&self) -> Option<ResourceStoreWrite<'_, T>> {
        let ty = TypeId::of::<T>();
        let cell = self
            .map
            .get(&ty)?
            .downcast_ref::<ResourceStoreCell<T>>()
            .expect("Downcast error");
        Some(ResourceStoreWrite::new(cell))
    }
}

/// Resources container.
#[derive(Default)]
pub struct Resources {
    internal: UnsafeResources,
    // marker to make `Resources` !Send and !Sync
    _not_send_sync: PhantomData<*const u8>,
}

impl Resources {
    /*/// Creates an accessor to resources which are Send and Sync and can be sent
    /// safely between threads.
    pub fn sync(&mut self) -> SyncResources {
        SyncResources {
            internal: &self.internal,
        }
    }*/

    /// Inserts an instance of `T` into the store with the given id
    pub fn insert_with_id<T: Resource>(&mut self, id: ResourceId, value: T) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.insert(id, value);
        }
    }

    /// Inserts the instance of `T` into the store.
    pub fn insert<T: Resource>(&mut self, value: T) {
        self.insert_with_id(ResourceId::Global, value);
    }

    /// Register resource type that can be created from id on demand.
    pub fn insert_managed<T: Resource, F: 'static + Fn(&ResourceId) -> T>(&mut self, build: F) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.create_managed(Some(Box::new(build)));
        }
    }

    /// Removes the instance of `T` with the given id from this store if it exists.    
    pub fn remove_with_id<T: Resource>(&mut self, id: &ResourceId) -> Option<T> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.remove::<T>(id) }
    }

    /// Removes the type `T` from this store if it exists.    
    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        self.remove_with_id::<T>(&ResourceId::Global)
    }

    pub fn get_store<T: Resource>(&self) -> Option<ResourceStoreRead<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.read_store::<T>() }
    }

    pub fn get_store_mut<T: Resource>(&self) -> Option<ResourceStoreWrite<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.write_store::<T>() }
    }

    pub fn get<T: Resource>(&self) -> Result<ResourceRead<'_, T>, ECSError> {
        self.get_with_id::<T>(&ResourceId::Global)
    }

    pub fn get_with_id<T: Resource>(&self, id: &ResourceId) -> Result<ResourceRead<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?
            .get_with_id(id)
    }

    pub fn get_with_ids<'i, T: Resource, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiRead<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .get_with_ids(ids)
    }

    pub fn get_mut<T: Resource>(&self) -> Result<ResourceWrite<'_, T>, ECSError> {
        self.get_mut_with_id::<T>(&ResourceId::Global)
    }

    pub fn get_mut_with_id<T: Resource>(&self, id: &ResourceId) -> Result<ResourceWrite<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?
            .get_mut_with_id(id)
    }

    pub fn get_mut_with_ids<'i, T: Resource, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiWrite<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .get_mut_with_ids(ids)
    }

    pub fn get_handle<T: Resource>(&self, id: &ResourceId) -> Result<ResourceHandle<T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .get_handle(id)
    }

    pub fn at<T: Resource>(&self, handle: &ResourceHandle<T>) -> Result<ResourceRead<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .at(handle)
    }

    pub fn at_mut<T: Resource>(&self, handle: &ResourceHandle<T>) -> Result<ResourceWrite<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .at_mut(handle)
    }
}
