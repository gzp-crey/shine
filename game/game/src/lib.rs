#![feature(async_closure)]

pub mod assets;
pub mod input;
pub mod render;

mod error;
pub use self::error::*;
mod config;
pub use self::config::*;
mod gamerender;
pub use self::gamerender::*;

pub use wgpu;
