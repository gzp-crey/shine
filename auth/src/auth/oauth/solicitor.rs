use crate::session::UserId;
use oxide_auth::endpoint::{OwnerConsent, OwnerSolicitor, PreGrant};
use oxide_auth_actix::{OAuthRequest, OAuthResponse, WebError};
use std::sync::Arc;
use std::rc::Rc;
use tera::Tera;

/// Perform authorization request with a signed in user
pub struct RequestWithAuthorizedUser {
    tera: Rc<Tera>,
    user: UserId,
}

impl RequestWithAuthorizedUser {
    pub fn new(tera: Rc<Tera>, user: UserId) -> RequestWithAuthorizedUser {
        RequestWithAuthorizedUser { tera, user }
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for RequestWithAuthorizedUser {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        let mut context = tera::Context::new();
        context.insert("client_id", &grant.client_id);
        context.insert("redirect_uri", &grant.redirect_uri.to_string());
        context.insert("scope", &grant.scope.to_string());
        context.insert("user", &self.user.name());

        let html = match self.tera.render("auth/request_login.html", &context) {
            Ok(html) => html,
            Err(e) => {
                log::error!("Tera render error: {}", e);
                return OwnerConsent::Error(WebError::InternalError(None));
            }
        };

        OwnerConsent::InProgress(OAuthResponse::ok().content_type("text/html").unwrap().body(&html))
    }
}

/// Perform authorization request with user login
pub struct RequestWithUserLogin {
    tera: Rc<Tera>,
}

impl RequestWithUserLogin {
    pub fn new(tera: Rc<Tera>) -> Self {
        RequestWithUserLogin { tera }
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for RequestWithUserLogin {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        unimplemented!()
    }
}

pub struct AuthorizeUser {
    user: UserId,
}

impl AuthorizeUser {
    pub fn new(user: UserId) -> Self {
        AuthorizeUser { user }
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for AuthorizeUser {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        unimplemented!()
    }
}

