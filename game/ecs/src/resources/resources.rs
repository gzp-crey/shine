use crate::resources::ResourceStoreCell;
use crate::{
    resources::{
        Resource, ResourceConfig, ResourceHandle, ResourceId, ResourceMultiRead, ResourceMultiWrite, ResourceRead,
        ResourceStoreRead, ResourceStoreWrite, ResourceWrite,
    },
    ECSError,
};
use downcast_rs::{impl_downcast, Downcast};
use std::{
    any::{type_name, TypeId},
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
    store_map: HashMap<TypeId, Box<dyn GeneralResourceStoreCell>>,
}

impl UnsafeResources {
    /// # Safety
    /// Resources which are `!Send` must be retrieved or created only on the thread owning the resource
    unsafe fn register<T: Resource>(&mut self, config: Box<dyn ResourceConfig<Resource = T>>) {
        let ty = TypeId::of::<T>();
        // Managed store have to be registered using the register
        // function before instances of the resource can be added
        assert!(
            self.store_map.get(&ty).is_none(),
            "Resource store for {} already created",
            type_name::<T>()
        );
        self.store_map.insert(ty, Box::new(ResourceStoreCell::<T>::new(config)));
    }

    unsafe fn unregister<T: Resource>(&mut self) {
        let ty = TypeId::of::<T>();
        let _ = self.store_map.remove(&ty);
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or created only on the thread owning the resource
    unsafe fn insert<T: Resource>(&mut self, id: ResourceId, resource: T) -> Result<Option<T>, ECSError> {
        let ty = TypeId::of::<T>();
        let cell = self
            .store_map
            .get_mut(&ty)
            .ok_or_else(|| ECSError::ResourceTypeNotFound(type_name::<T>().into()))?
            .downcast_mut::<ResourceStoreCell<T>>()
            .expect("Downcast error");
        Ok(ResourceStoreWrite::new(cell).insert(id, resource))
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
            .store_map
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
            .store_map
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

    /// Register a new type of resource with the given managed configuration.
    pub fn register<T: Resource, TC: 'static + ResourceConfig<Resource = T>>(&mut self, config: TC) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.register::<T>(Box::new(config));
        }
    }

    /// Unregister and release all the resources of the given type. This operation also invalidates
    /// all the handles. The other type of references and accessors
    /// are not effected as they should not exist by the design of the API.
    pub fn unregister<T: Resource>(&mut self) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.unregister::<T>();
        }
    }

    /// Inserts a new instance of `T` with the given id into the store.
    /// If resource alread exists it is replaced and the old value is returned. All the handles
    /// are invalidated. The other type of references and accessors are not effected as they
    /// should not exist by the design of the API.
    pub fn insert_with_id<T: Resource>(&mut self, id: ResourceId, value: T) -> Result<Option<T>, ECSError> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.insert(id, value) }
    }

    /// Inserts the instance of `T` into the store.
    /// If resource alread exists it is replaced and the old value is returned. All the handles
    /// are invalidated. The other type of references and accessors are not effected as they
    /// should not exist by the design of the API.
    pub fn insert<T: Resource>(&mut self, value: T) -> Result<Option<T>, ECSError> {
        self.insert_with_id(ResourceId::Global, value)
    }

    /// Removes the instance of `T` with the given id from this store if it exists.
    /// All the handles
    /// are invalidated. The other type of references and accessors are not effected as they
    /// should not exist by the design of the API.   
    pub fn remove_with_id<T: Resource>(&mut self, id: &ResourceId) -> Option<T> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.remove::<T>(id) }
    }

    /// Removes the type `T` from this store if it exists. All the handles
    /// are invalidated. The other type of references and accessors are not effected as they
    /// should not exist by the design of the API.   
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
            .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.clone()))?
            .get_with_id(id)
    }

    pub fn get_with_ids<'i, T: Resource, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiRead<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(type_name::<T>().into()))?
            .get_with_ids(ids)
    }

    pub fn get_mut<T: Resource>(&self) -> Result<ResourceWrite<'_, T>, ECSError> {
        self.get_mut_with_id::<T>(&ResourceId::Global)
    }

    pub fn get_mut_with_id<T: Resource>(&self, id: &ResourceId) -> Result<ResourceWrite<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.clone()))?
            .get_mut_with_id(id)
    }

    pub fn get_mut_with_ids<'i, T: Resource, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiWrite<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(type_name::<T>().into()))?
            .get_mut_with_ids(ids)
    }

    pub fn get_handle<T: Resource>(&self, id: &ResourceId) -> Result<ResourceHandle<T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(type_name::<T>().into()))?
            .get_handle(id)
    }

    pub fn at<T: Resource>(&self, handle: &ResourceHandle<T>) -> Result<ResourceRead<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(type_name::<T>().into()))?
            .at(handle)
    }

    pub fn at_mut<T: Resource>(&self, handle: &ResourceHandle<T>) -> Result<ResourceWrite<'_, T>, ECSError> {
        self.get_store::<T>()
            .ok_or_else(|| ECSError::ResourceTypeNotFound(type_name::<T>().into()))?
            .at_mut(handle)
    }
}
