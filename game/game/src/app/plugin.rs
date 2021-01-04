use crate::{app::AppError, World};
use std::{borrow::Cow, future::Future, pin::Pin};

pub type PluginFuture<'a, P> = Pin<Box<dyn Future<Output = Result<P, AppError>> + 'a>>;

pub trait Plugin {
    fn name() -> Cow<'static, str>;

    fn init(self, world: &mut World) -> PluginFuture<()>
    where
        Self: Sized;

    fn deinit(world: &mut World) -> PluginFuture<()>
    where
        Self: Sized;
}
