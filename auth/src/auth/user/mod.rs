mod error;
mod userdb;

use super::State;
use crate::session::UserId;
use actix_session::Session;
use actix_web::{web, Error as ActixError, HttpResponse};
use actix_web_httpauth::extractors::basic::BasicAuth;
use serde::{Deserialize, Serialize};

pub use self::error::*;
pub use self::userdb::{test_az, IdentityConfig};

pub async fn login(session: Session, auth: BasicAuth, state: web::Data<State>) -> Result<HttpResponse, ActixError> {
    log::info!("login {:?}, {:?}", auth.user_id(), auth.password());
    UserId::new(auth.user_id().to_owned().to_string(), "a".to_string(), vec![]).to_session(&session)?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(Serialize, Deserialize)]
pub struct Registration {
    name: String,
    email: String,
    password: String,
}

pub async fn register(
    session: Session,
    registration: web::Json<Registration>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    log::info!("register {:?}, {:?}", registration.name, registration.password);
    UserId::new("a".to_owned(), registration.name.clone(), vec![]).to_session(&session)?;
    //async { state.userdb.insert_one() }.await?;
    Ok(HttpResponse::Ok().finish())
}
