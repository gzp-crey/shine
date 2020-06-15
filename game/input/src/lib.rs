pub mod guestures;
pub use self::guestures::{Guesture, GuestureManager};

mod manager;
pub use self::manager::*;
mod state;
pub use self::state::*;

/// Id of an input controller (Ex. button, axis, etc.)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputId(u32);

impl InputId {
    pub const fn new(code: u32) -> InputId {
        InputId(code)
    }

    pub fn id(self) -> u32 {
        self.0
    }
}

pub struct InputIdGenerator(u32);

impl InputIdGenerator {
    pub fn new() -> InputIdGenerator {
        InputIdGenerator(0)
    }

    pub fn with_start(start: u32) -> InputIdGenerator {
        InputIdGenerator(start)
    }

    pub fn next(&mut self) -> InputId {
        self.0 += 1;
        InputId(self.0)
    }
}
