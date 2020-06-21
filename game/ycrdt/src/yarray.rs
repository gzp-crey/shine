use crate::crdt::{ItemId, ItemStore};

pub struct YArray<C> {
    store: ItemStore<C>,
    start: Option<ItemId>,
}

impl<C> YArray<C> {
    pub fn new() -> YArray<C> {
        YArray {
            store: ItemStore::new(),
            start: None,
        }
    }

    pub fn insert(&mut self, at: usize, value: C) {
        unimplemented!()
    }

    pub fn remove(&mut self, at: usize) -> Option<C> {
        unimplemented!()
    }
}
