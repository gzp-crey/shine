mod system;
pub use self::system::*;
mod schedule;
pub use self::schedule::*;
mod fn_system;
pub use self::fn_system::*;

mod resource_claim;
pub use self::resource_claim::*;
mod resource_query;
pub use self::resource_query::*;

pub mod prelude {
    pub use super::{
        FetchResource, IntoResourceClaim, IntoSystem, IntoSystemBuilder, MultiRes, MultiResMut, Res, ResMut,
        ResourceQuery, Schedule, WithMultiRes, WithMultiResMut,
    };
}
