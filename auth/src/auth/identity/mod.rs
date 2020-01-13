mod error;
mod identity_manager;
mod identityentry;
mod loginentry;

use super::State;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};
use shine_core::authheader::BasicAuth;
use shine_core::session::{IdentityCookie, Session, SessionKey, UserId};
use shine_core::siteinfo::SiteInfo;

pub use self::error::*;
pub use self::identity_manager::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    password_pepper: String,
    user_id_secret: String,
    storage_account: String,
    storage_account_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registration {
    name: String,
    password: String,
    email: Option<String>,
}

pub async fn register_user(
    session: Session,
    site: SiteInfo,
    registration: web::Json<Registration>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    log::info!("register {:?} {:?}", registration, site);
    IdentityCookie::clear(&session);

    let Registration { name, password, email } = registration.into_inner();
    let identity = state.identity_db().create_user(name, email, password).await?;
    let login = state.identity_db().create_login(&identity, site).await?;

    UserId::from(identity).to_session(&session)?;
    SessionKey::from(login).to_session(&session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn login_basicauth(
    session: Session,
    site: SiteInfo,
    auth: BasicAuth,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let (user_id, password) = (auth.user_id(), auth.password());
    log::info!("login {:?}, {:?}, {:?}", user_id, password, site);
    IdentityCookie::clear(&session);

    let identity = state.identity_db().find_by_login(&user_id, password.as_deref()).await?;
    let login = state.identity_db().create_login(&identity, site).await?;

    UserId::from(identity).to_session(&session)?;
    SessionKey::from(login).to_session(&session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn refresh_session(session: Session, site: SiteInfo, state: web::Data<State>) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&session);
    let user_id = UserId::from_session(&session);
    log::info!("refresh session {:?}, {:?}, {:?}", user_id, session_key, site);
    //IdentityCookie::clear(&session);

    Ok(HttpResponse::Ok().finish())
}
