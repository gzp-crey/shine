mod error;
mod identity;
mod identitydb;
mod siteinfo;

use super::State;
use crate::authheader::BasicAuth;
use crate::session::UserId;
use actix_session::Session;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};

pub use self::error::*;
pub use self::identitydb::*;
pub use self::siteinfo::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    password_pepper: String,
    user_id_secret: String,
    login_key_secret: String,
    storage_account: String,
    storage_account_key: String,
}

pub async fn login(session: Session, auth: BasicAuth, state: web::Data<State>) -> Result<HttpResponse, ActixError> {
    log::info!("login {:?}, {:?}", auth.user_id(), auth.password());
    let user = state
        .identity_db()
        .find_by_login(&auth.user_id(), auth.password().as_deref())
        .await
        .or_else(|err| {
            UserId::clear_session(&session);
            Err(HttpResponse::Forbidden().finish())
        })?;

    UserId::from(user).to_session(&session)?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registration {
    name: String,
    password: String,
    email: Option<String>,
}

pub async fn register(
    session: Session,
    registration: web::Json<Registration>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    log::info!("register {:?}", registration);
    let Registration { name, password, email } = registration.into_inner();
    let identity = state.identity_db().create(name, email, password).await?;
    let user = UserId::from(identity);
    user.to_session(&session)?;
    Ok(HttpResponse::Ok().finish())
}
