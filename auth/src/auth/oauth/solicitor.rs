use super::{OAuthAuthorizer, OAuthIssuer, OAuthRegistrar, OAuthScope, State};
use shine_core::session::UserId;
use oxide_auth::{
    endpoint::{Endpoint, OAuthError, OwnerConsent, OwnerSolicitor, PreGrant, Scopes, Template},
    frontends::simple::endpoint::{ResponseCreator, Vacant},
    primitives::authorizer::Authorizer,
    primitives::issuer::Issuer,
    primitives::registrar::Registrar,
};
use oxide_auth_actix::{OAuthRequest, OAuthResponse, WebError};

pub struct OAuthFlow<S> {
    registrar: OAuthRegistrar,
    authorizer: OAuthAuthorizer,
    issuer: OAuthIssuer,
    scopes: OAuthScope,
    response: Vacant,
    solicitor: S,
}

impl<S> OAuthFlow<S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    pub fn new(state: State, solicitor: S) -> Self {
        OAuthFlow {
            registrar: OAuthRegistrar::new(state.clone()),
            authorizer: OAuthAuthorizer::new(state.clone()),
            issuer: OAuthIssuer::new(state.clone()),
            scopes: OAuthScope::new(state.clone()),
            response: Vacant,
            solicitor,
        }
    }
}

impl<S> Endpoint<OAuthRequest> for OAuthFlow<S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    type Error = WebError;

    fn registrar(&self) -> Option<&dyn Registrar> {
        Some(&self.registrar)
    }

    fn authorizer_mut(&mut self) -> Option<&mut dyn Authorizer> {
        Some(&mut self.authorizer)
    }

    fn issuer_mut(&mut self) -> Option<&mut dyn Issuer> {
        Some(&mut self.issuer)
    }

    fn owner_solicitor(&mut self) -> Option<&mut dyn OwnerSolicitor<OAuthRequest>> {
        Some(&mut self.solicitor)
    }

    fn scopes(&mut self) -> Option<&mut dyn Scopes<OAuthRequest>> {
        Some(&mut self.scopes)
    }

    fn response(&mut self, request: &mut OAuthRequest, kind: Template) -> Result<OAuthResponse, WebError> {
        Ok(self.response.create(request, kind))
    }

    fn error(&mut self, err: OAuthError) -> WebError {
        WebError::Endpoint(err)
    }

    fn web_error(&mut self, err: WebError) -> WebError {
        err
    }
}

/// Perform authorization request with a signed in user
pub struct RequestWithAuthorizedUser {
    state: State,
    user: UserId,
}

impl RequestWithAuthorizedUser {
    pub fn solicite(state: &State, user: UserId) -> OAuthFlow<Self> {
        OAuthFlow::new(
            state.clone(),
            RequestWithAuthorizedUser {
                state: state.clone(),
                user,
            },
        )
    }
}

impl OwnerSolicitor<OAuthRequest> for RequestWithAuthorizedUser {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        let mut context = tera::Context::new();
        context.insert("client_id", &grant.client_id);
        context.insert("redirect_uri", &grant.redirect_uri.to_string());
        context.insert("scope", &grant.scope.to_string());
        context.insert("user", &self.user.name());

        let tera = self.state.tera();
        let html = match tera.render("auth/request_login.html", &context) {
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
    state: State,
}

impl RequestWithUserLogin {
    pub fn solicite(state: &State) -> OAuthFlow<Self> {
        OAuthFlow::new(state.clone(), RequestWithUserLogin { state: state.clone() })
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for RequestWithUserLogin {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        unimplemented!()
    }
}

/// Perform authorization request with user login
pub struct AuthorizeUser {
    state: State,
    user: UserId,
}

impl AuthorizeUser {
    pub fn solicite(state: &State, user: UserId) -> OAuthFlow<Self> {
        OAuthFlow::new(
            state.clone(),
            AuthorizeUser {
                state: state.clone(),
                user,
            },
        )
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for AuthorizeUser {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        unimplemented!()
    }
}

/// Validate token
pub struct ValidateToken {
    state: State,
}

impl ValidateToken {
    pub fn solicite(state: &State) -> OAuthFlow<Self> {
        OAuthFlow::new(state.clone(), ValidateToken { state: state.clone() })
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for ValidateToken {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        unimplemented!()
    }
}

/// Refresh token
pub struct RefreshToken {
    state: State,
}

impl RefreshToken {
    pub fn solicite(state: &State) -> OAuthFlow<Self> {
        OAuthFlow::new(state.clone(), RefreshToken { state: state.clone() })
    }
}

impl<'a> OwnerSolicitor<OAuthRequest> for RefreshToken {
    fn check_consent(&mut self, _request: &mut OAuthRequest, grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        unimplemented!()
    }
}
