mod authorizer;
mod issuer;
mod regsitrar;
mod solicitor;

use actix::{Actor, Addr, Context, Handler, MailboxError};
use actix_web::{web, HttpRequest};
use futures::compat::{Compat, Compat01As03};
use futures::TryFuture;
use oxide_auth::{
    endpoint::{Endpoint, OAuthError, OwnerConsent, OwnerSolicitor, PreGrant, Scopes, Template},
    frontends::simple::endpoint::{FnSolicitor, Vacant},
    primitives::authorizer::Authorizer,
    primitives::grant::Grant,
    primitives::issuer::{IssuedToken, Issuer},
    primitives::prelude::{AuthMap, Client, RandomGenerator, TokenMap},
    primitives::registrar::Registrar,
    primitives::scope::Scope,
};
use oxide_auth_actix::{Authorize, OAuthMessage, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};
use std::sync::Arc;
use tera::Tera;

use authorizer::MyAuthorizer;
use regsitrar::MyRegistrar;

enum AuthError {
    Web(WebError),
    OAuth(OAuthError),
    Internal(String),
}

struct MyIssuer {
    inner: TokenMap<RandomGenerator>,
}

impl Issuer for MyIssuer {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        log::info!("issue");
        self.inner.issue(grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        log::info!("recover_token");
        self.inner.recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        log::info!("recover_refresh");
        self.inner.recover_refresh(token)
    }
}

fn request_login(tera: &Tera, _request: &mut OAuthRequest, pre_grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
    /*let mut context = tera::Context::new();
    context.insert("client_id", &pre_grant.client_id);
    context.insert("redirect_uri", &pre_grant.redirect_uri.to_string());
    context.insert("scope", &pre_grant.scope.to_string());

    let html = match tera.render("auth/request_login.html", &context) {
        Ok(html) => html,
        Err(_) => {
            return OwnerConsent::Error(WebError::Mailbox);
        }
    };*/

    OwnerConsent::InProgress(OAuthResponse::ok().content_type("text/html").unwrap().body(""))
}

struct AuthState {
    registrar: MyRegistrar,
    authorizer: MyAuthorizer,
    issuer: MyIssuer,
    scopes: Vec<Scope>,
}

struct AuthStateWithSolicitor<'a, S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    inner: &'a mut AuthState,
    solicitor: S,
}

impl Endpoint<OAuthRequest> for AuthState {
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
        None
    }

    fn scopes(&mut self) -> Option<&mut dyn Scopes<OAuthRequest>> {
        Some(&mut self.scopes)
    }

    fn response(&mut self, request: &mut OAuthRequest, kind: Template) -> Result<OAuthResponse, Self::Error> {
        unimplemented!()
        //Ok(OAuthResponse::ok(request, kind))
    }

    fn error(&mut self, err: OAuthError) -> Self::Error {
        WebError::Mailbox
        //AuthError::OAuth(err)
    }

    fn web_error(&mut self, err: WebError) -> Self::Error {
        err
        //AuthError::Web(err)
    }
}

impl<'a, S> Endpoint<OAuthRequest> for AuthStateWithSolicitor<'a, S>
where
    S: OwnerSolicitor<OAuthRequest>,
{
    type Error = WebError;

    fn registrar(&self) -> Option<&dyn Registrar> {
        self.inner.registrar()
    }

    fn authorizer_mut(&mut self) -> Option<&mut dyn Authorizer> {
        self.inner.authorizer_mut()
    }

    fn issuer_mut(&mut self) -> Option<&mut dyn Issuer> {
        self.inner.issuer_mut()
    }

    fn owner_solicitor(&mut self) -> Option<&mut dyn OwnerSolicitor<OAuthRequest>> {
        Some(&mut self.solicitor)
    }

    fn scopes(&mut self) -> Option<&mut dyn Scopes<OAuthRequest>> {
        self.inner.scopes()
    }

    fn response(&mut self, request: &mut OAuthRequest, kind: Template) -> Result<OAuthResponse, Self::Error> {
        self.inner.response(request, kind)
    }

    fn error(&mut self, err: OAuthError) -> Self::Error {
        self.inner.error(err)
    }

    fn web_error(&mut self, err: WebError) -> Self::Error {
        self.inner.web_error(err)
    }
}

impl AuthState {
    fn new() -> Self {
        let issuer = TokenMap::new(RandomGenerator::new(16));
        let issuer = MyIssuer { inner: issuer };

        AuthState {
            registrar: MyRegistrar::new(),
            authorizer: MyAuthorizer::new(),
            issuer,
            scopes: vec!["default-scope".parse().unwrap()],
        }
    }

    fn with_solicitor<S: OwnerSolicitor<OAuthRequest>>(&mut self, solicitor: S) -> AuthStateWithSolicitor<'_, S> {
        AuthStateWithSolicitor { inner: self, solicitor }
    }
}

impl Actor for AuthState {
    type Context = Context<Self>;
}

impl<Op, S> Handler<OAuthMessage<Op, S>> for AuthState
where
    Op: OAuthOperation,
    S: OwnerSolicitor<OAuthRequest>,
{
    type Result = Result<Op::Item, Op::Error>;

    fn handle(&mut self, msg: OAuthMessage<Op, S>, _: &mut Self::Context) -> Self::Result {
        let (op, solicitor) = msg.into_inner();
        let r = op.run(&mut self.with_solicitor(solicitor));
        unimplemented!();
    }
}

struct State {
    auth: Addr<AuthState>,
    tera: Arc<Tera>,
}

impl State {
    fn new() -> Result<State, String> {
        let tera = match Tera::new("tera_web/auth/**/*") {
            Ok(t) => t,
            Err(e) => return Err(format!("Tera template parsing error(s): {}", e)),
        };

        Ok(State {
            auth: AuthState::new().start(),
            tera: Arc::new(tera),
        })
    }
}

fn get_authorization(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("get_authorization");
    let tera = state.tera.clone();
    let solicitor = FnSolicitor(move |request: &mut _, pre_grant: &_| request_login(&tera, request, pre_grant));
    Compat01As03::new(state.auth.send(Authorize(oath_req).wrap(solicitor)))
}

fn post_authorization(
    req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_authorization");
    // Some authentication should be performed here in production cases
    Compat01As03::new(state.auth.send(Authorize(oath_req).wrap(Vacant)))
}

fn post_token(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_token");
    Compat01As03::new(state.auth.send(Token(oath_req).wrap(Vacant)))
}

fn post_refresh(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    Compat01As03::new(state.auth.send(Refresh(oath_req).wrap(Vacant)))
}

pub fn configure_service(cfg: &mut web::ServiceConfig) {
    let data = web::Data::new(State::new());
    cfg.service(
        web::scope("/api/auth")
            .register_data(data.clone())
            .service(
                web::resource("authorize")
                    .route(web::get().to_async(|a, b| Compat::new(get_authorization(a, b))))
                    .route(web::post().to_async(|a, b, c| Compat::new(post_authorization(a, b, c)))),
            )
            .service(web::resource("refresh").route(web::post().to_async(|a, b| Compat::new(post_refresh(a, b)))))
            .service(web::resource("token").route(web::post().to_async(|a, b| Compat::new(post_token(a, b))))),
    );
}
