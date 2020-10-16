mod error;
pub mod resources;
pub mod scheduler;

pub use self::error::*;
pub use hecs;

pub mod prelude {
    pub use super::scheduler::{IntoSystem, IntoSystemBuilder};
    pub use super::scheduler::{WithTag, WithTagMut};
}
