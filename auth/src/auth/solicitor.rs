use oxide_auth::endpoint::{OwnerConsent, OwnerSolicitor, PreGrant};
use oxide_auth_actix::{OAuthRequest, OAuthResponse, WebError};
use std::sync::Arc;
use tera::Tera;

pub struct RequestAuthorizeWithLogin {
    tera: Arc<Tera>,
}

impl RequestAuthorizeWithLogin {
    pub fn new(tera: Arc<Tera>) -> Self {
        RequestAuthorizeWithLogin { tera }
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for RequestAuthorizeWithLogin {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        let mut context = tera::Context::new();
        context.insert("client_id", &grant.client_id);
        context.insert("redirect_uri", &grant.redirect_uri.to_string());
        context.insert("scope", &grant.scope.to_string());

        let html = match self.tera.render("auth/request_login.html", &context) {
            Ok(html) => html,
            Err(e) => {
                log::error!("Tera render error: {}", e);
                return OwnerConsent::Error(WebError::Mailbox);
            }
        };

        OwnerConsent::InProgress(OAuthResponse::ok().content_type("text/html").unwrap().body(&html))
    }
}

pub struct AuthorizeWithLogin {
    tera: Arc<Tera>,
}

impl AuthorizeWithLogin {
    pub fn new(tera: Arc<Tera>) -> Self {
        AuthorizeWithLogin { tera }
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for AuthorizeWithLogin {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        let mut context = tera::Context::new();
        context.insert("client_id", &grant.client_id);
        context.insert("redirect_uri", &grant.redirect_uri.to_string());
        context.insert("scope", &grant.scope.to_string());

        let html = match self.tera.render("auth/request_login.html", &context) {
            Ok(html) => html,
            Err(_) => {
                return OwnerConsent::Error(WebError::Mailbox);
            }
        };

        OwnerConsent::InProgress(OAuthResponse::ok().content_type("text/html").unwrap().body(&html))
    }
}
