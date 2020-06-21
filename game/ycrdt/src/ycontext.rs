use crate::crdt::ClientId;

/// Store client_id and encoder and peer informations
pub struct YContext {
    client_id: ClientId,
}

impl YContext {
    pub fn new(client_id: ClientId) -> YContext {
        YContext { client_id }
    }
}
