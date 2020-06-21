use crate::{crdt::ClientId, YArray, YContext};

pub struct YArrayDoc<C> {
    context: YContext,
    array: YArray<C>,
}

impl<C> YArrayDoc<C> {
    pub fn new(context: YContext) -> YArrayDoc<C> {
        YArrayDoc {
            context,
            array: YArray::new(),
        }
    }

    pub fn insert(&mut self, at: usize, value: C) {
        unimplemented!()
    }

    pub fn remove(&mut self, at: usize) {
        unimplemented!()
    }
}
