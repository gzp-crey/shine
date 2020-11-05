#![feature(drain_filter)]
#![feature(async_closure)]
#![feature(get_mut_unchecked)]
#![feature(type_name_of_val)]
#![feature(min_const_generics)]
#![feature(or_insert_with_key)]
#![allow(clippy::module_inception)]
#![allow(clippy::match_like_matches_macro)]

pub mod core;
mod error;
pub use self::error::*;
pub mod resources;
pub mod utils;
