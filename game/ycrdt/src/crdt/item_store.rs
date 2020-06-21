use crate::crdt::{ClientId, ItemList};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub struct ItemStore<C> {
    clients: HashMap<ClientId, ItemList>,
    content: Vec<C>,
}

impl<C> ItemStore<C> {
    pub fn new() -> ItemStore<C> {
        ItemStore {
            clients: HashMap::new(),
            content: Vec::new(),
        }
    }
}
