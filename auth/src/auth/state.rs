use super::identity::IdentityDB;
use super::oauth::AuthState;
use super::{AuthConfig, AuthError};
use actix::{Actor, Addr, SystemRunner};
use futures::future::{FutureExt, TryFutureExt};
use std::sync::Arc;
use tera::Tera;

pub struct State {
    pub auth: Addr<AuthState>,
    pub tera: Arc<Tera>,
    pub identity_db: Arc<IdentityDB>,
}

impl State {
    pub fn new(sys: &mut SystemRunner, config: &AuthConfig) -> Result<State, AuthError> {
        let tera = Tera::new("tera_web/**/*")?;
        let identity_db = sys.block_on(IdentityDB::new(&config.identity).boxed_local().compat())?;

        Ok(State {
            auth: AuthState::new().start(),
            tera: Arc::new(tera),
            identity_db: Arc::new(identity_db),
        })
    }
}
