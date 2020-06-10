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
