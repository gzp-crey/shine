use crate::{app::AppError, World};
use std::{future::Future, pin::Pin};

pub type GameFuture<'a, R> = Pin<Box<dyn Future<Output = R> + 'a>>;

/// Source of a game
pub trait GameSource {
    fn build(self) -> Result<Box<dyn GameLifecycle>, AppError>
    where
        Self: Sized;
}

/// Manage the lifecyle of a game by tearing up/down the world.
pub trait GameLifecycle: 'static + Send + Sync {
    fn create<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>>;
    fn destroy<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>>;
}
