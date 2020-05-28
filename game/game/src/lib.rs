#![feature(async_closure)]
#![feature(str_strip)]

pub mod assets;
pub mod input;
pub mod render;
pub mod world;

mod error;
pub use self::error::*;
mod config;
pub use self::config::*;
mod scheduleset;
pub use self::scheduleset::*;
mod gamerender;
pub use self::gamerender::*;

pub use wgpu;
