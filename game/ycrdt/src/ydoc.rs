use crate::{crdt::ClientId, YArray};



/// Store client_id and encoder and peer informations
pub struct YContext {
    client_id: ClientId,
}

impl YContext {
    fn new(client_id: ClientId) -> YContext {
        YContext {
            client_id
        }
    }
}

pub struct YArrayDoc<C> {
    context: YContext,
    array: YArray<C>,
}

impl<C> YArrayDoc<C> {
    pub fn new(context: Ycontext) -> YArrayDoc {
        YArrayDoc {
            context,
            array: YArray::new()
        }
    }

    pub fn add(&mut self, at : usize, value: C) {
        unimplemented!
    }

    pub fn remove(&mut self, at: usize) {
        unimplemented!
    }
}
