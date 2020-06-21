use crate::crdt::ItemStore;

pub struct YArray/*<'d>*/ {
    store: ItemStore<C>,
    start: Option<ItemId>,
}

impl YArray<C> {
    pub fn new() -> YArray {
        YArray {
            start: ItemStore::new(),
            start: None
        }
    }

    pub fn add(&mut self, at: usize, value: C) {
        unimplemented!()
    }

    pub fn remove(&mut self, at: usize) -> Option<C> {
        unimplemented!()
    }
}