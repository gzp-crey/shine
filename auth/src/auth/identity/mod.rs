mod error;
mod manager;
//mod sessionentry;

use super::State;
use actix_web::{web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};
use shine_core::authheader::BasicAuth;
use shine_core::session::{IdentityCookie, IdentitySession, SessionKey, UserId};
use shine_core::siteinfo::SiteInfo;

pub use self::error::*;
pub use self::manager::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    password_pepper: String,
    identity_id_secret: String,
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
    identity_session: IdentitySession,
    site: SiteInfo,
    registration: web::Json<Registration>,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    log::info!("register {:?} {:?}", registration, site);
    IdentityCookie::clear(&identity_session);

    let Registration { name, password, email } = registration.into_inner();
    let identity = state.identity_db().create_user(name, email, password).await?;
    let session = state.identity_db().create_session(&identity, site).await?;

    UserId::from(identity).to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn login_basic_auth(
    identity_session: IdentitySession,
    site: SiteInfo,
    auth: BasicAuth,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let (user_id, password) = (auth.user_id(), auth.password());
    log::info!("login {:?}, {:?}, {:?}", user_id, password, site);
    IdentityCookie::clear(&identity_session);

    let identity = state
        .identity_db()
        .find_identity_by_name_email(&user_id, password.as_deref())
        .await?;
    let session = state.identity_db().create_session(&identity, site).await?;

    UserId::from(identity).to_session(&identity_session)?;
    SessionKey::from(session).to_session(&identity_session)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn refresh_session(
    identity_session: IdentitySession,
    site: SiteInfo,
    state: web::Data<State>,
) -> Result<HttpResponse, ActixError> {
    let session_key = SessionKey::from_session(&identity_session)?.ok_or(IdentityError::SessionRequired)?;
    let user_id = UserId::from_session(&identity_session)?.ok_or(IdentityError::SessionRequired)?;
    log::info!("refresh session {:?}, {:?}, {:?}", user_id, session_key, site);

    IdentityCookie::clear(&identity_session);
    match state.identity_db().refresh_session(session_key.key(), &site).await {
        Ok((identity, session)) => {
            UserId::from(identity).to_session(&identity_session)?;
            SessionKey::from(session).to_session(&identity_session)?;
            Ok(HttpResponse::Ok().finish())
        }
        Err(e) => {
            IdentityCookie::clear(&identity_session);
            Err(e.into())
        }
    }
}
