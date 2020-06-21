/// Client id for unique identification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClientId(u32);

impl From<u32> for ClientId {
    fn from(c: u32) -> ClientId {
        ClientId(c)
    }
}

/// Operation counter clock
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock(u32);

impl Clock {
    pub fn new() -> Clock {
        Clock(0)
    }

    pub fn increment(self) -> Clock {
        Clock(self.0 + 1)
    }
}

impl From<u32> for Clock {
    fn from(c: u32) -> Clock {
        Clock(c)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Id {
    pub client: ClientId,
    pub clock: Clock,
}

impl Id {
    pub fn new(client: ClientId, clock: Clock) -> Id {
        Id { client, clock }
    }

    pub fn with_clock(self, clock: Clock) -> Id {
        Id { clock: clock, ..self }
    }
}
