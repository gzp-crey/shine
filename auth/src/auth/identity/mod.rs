mod error;
mod identitydb;
mod identityentry;
mod loginentry;
mod siteinfo;

use super::State;
use crate::authheader::BasicAuth;
use crate::session::{IdentityCookie, SessionKey, UserId};
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
    let site = SiteInfo {
        ip: " ".to_string(),
        agent: " ".to_string(),
    };
    let (user_id, password) = (auth.user_id(), auth.password());
    log::info!("login {:?}, {:?}, {:?}", user_id, password, site);
    IdentityCookie::clear(&session);

    let identity = state.identity_db().find_by_login(&user_id, password.as_deref()).await?;
    let login = state.identity_db().create_login(&identity, site).await?;

    UserId::from(identity).to_session(&session)?;
    SessionKey::from(login).to_session(&session)?;

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
    let site = SiteInfo {
        ip: " ".to_string(),
        agent: " ".to_string(),
    };
    log::info!("register {:?} {:?}", registration, site);
    IdentityCookie::clear(&session);

    let Registration { name, password, email } = registration.into_inner();
    let identity = state.identity_db().create(name, email, password).await?;
    let login = state.identity_db().create_login(&identity, site).await?;

    UserId::from(identity).to_session(&session)?;
    SessionKey::from(login).to_session(&session)?;

    Ok(HttpResponse::Ok().finish())
}
