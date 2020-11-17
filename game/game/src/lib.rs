#![feature(async_closure)]
#![feature(clamp)]
#![allow(clippy::match_like_matches_macro)]

mod world;
pub use self::world::*;

pub mod app;
pub mod assets;
//pub mod components;
//pub mod game;
pub mod input;
//pub mod render;

pub use wgpu;
