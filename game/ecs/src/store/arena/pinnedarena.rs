use log::trace;
use std::marker::PhantomData;

/// Arena allocator that ensure stable memory location for the stored objects.
/// todo: Allocate pages of system memory and manage a free list similar to the indexed version
pub struct PinnedArena<T> {
    size: usize,
    _ph: PhantomData<T>,
}

impl<T> PinnedArena<T> {
    pub fn new() -> PinnedArena<T> {
        PinnedArena {
            size: 0,
            _ph: PhantomData,
        }
    }

    pub fn allocate(&mut self, data: T) -> &mut T {
        self.size += 1;
        trace!("size after allocation: {}", self.size);
        let b = Box::new(data);
        unsafe { &mut *Box::into_raw(b) }
    }

    pub fn deallocate(&mut self, data: &mut T) {
        self.size -= 1;
        trace!("size after deallocation: {}", self.size);
        unsafe { Box::from_raw(data as *mut T) };
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

impl<T> Default for PinnedArena<T> {
    fn default() -> Self {
        Self::new()
    }
}
