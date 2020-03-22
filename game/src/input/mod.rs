pub mod guestures;
pub use self::guestures::Guesture;

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

/// Trait to map OS events into state change
pub trait InputMapper {
    type InputEvent;

    fn map_event(&self, event: &Self::InputEvent, state: &mut InputState);
}
