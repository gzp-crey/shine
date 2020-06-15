#![feature(async_closure)]
#![feature(str_strip)]

pub mod assets;
pub mod components;
pub mod input;
pub mod render;
pub mod world;

mod error;
pub use self::error::*;
mod config;
pub use self::config::*;
mod scheduleset;
pub use self::scheduleset::*;
mod game_view;
pub use self::game_view::*;

pub use wgpu;
