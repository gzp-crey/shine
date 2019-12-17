use super::State;
use oxide_auth::{endpoint::Scopes, primitives::scope::Scope};
use oxide_auth_actix::OAuthRequest;
use std::rc::Rc;

pub struct OAuthScope {
    state: Rc<State>,
    scopes: Vec<Scope>,
}

impl OAuthScope {
    pub fn new(state: Rc<State>) -> Self {
        let scopes = vec!["default-scope".parse().unwrap()];
        OAuthScope { state, scopes }
    }
}

impl Scopes<OAuthRequest> for OAuthScope {
    fn scopes(&mut self, _request: &mut OAuthRequest) -> &[Scope] {
        &self.scopes
    }
}
