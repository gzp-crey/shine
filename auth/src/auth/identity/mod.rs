mod error;
mod identity;
mod identitydb;

use super::State;
use crate::authheader::BasicAuth;
use crate::session::UserId;
use actix_session::Session;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

pub use self::error::*;
pub use self::identitydb::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    password_pepper: String,
    storage_account: String,
    storage_account_key: String,
}

pub async fn login(session: Session, auth: BasicAuth, state: web::Data<Rc<State>>) -> Result<HttpResponse, ActixError> {
    log::info!("login {:?}, {:?}", auth.user_id(), auth.password());
    UserId::new(auth.user_id().to_owned().to_string(), "a".to_string(), vec![]).to_session(&session)?;
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
    let user = state.identity_db().create(name, email, password).await?;
    let user = UserId::from(user);
    user.to_session(&session)?;
    Ok(HttpResponse::Ok().finish())
}
