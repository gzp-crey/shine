use actix::{Actor, Addr, Context, Handler, MailboxError};
use actix_web::{web, HttpRequest};
use futures::compat::{Compat, Compat01As03};
use futures::{Future, TryFuture};
use oxide_auth::{
    endpoint::{Endpoint, OwnerConsent, OwnerSolicitor, PreGrant},
    frontends::simple::endpoint::{ErrorInto, FnSolicitor, Generic},
    primitives::authorizer::Authorizer,
    primitives::grant::Grant,
    primitives::issuer::{IssuedToken, Issuer},
    primitives::prelude::{AuthMap, Client, ClientMap, ClientUrl, RandomGenerator, Scope, TokenMap},
    primitives::registrar::{BoundClient, Registrar, RegistrarError},
};
use oxide_auth_actix::{Authorize, OAuthMessage, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};

struct MyRegistrar {
    inner: ClientMap,
}

impl Registrar for MyRegistrar {
    fn bound_redirect<'a>(&self, bound: ClientUrl<'a>) -> Result<BoundClient<'a>, RegistrarError> {
        log::info!("bound_redirect");
        self.inner.bound_redirect(bound)
    }

    /// Always overrides the scope with a default scope.
    fn negotiate(&self, bound: BoundClient, scope: Option<Scope>) -> Result<PreGrant, RegistrarError> {
        log::info!("negotiate");
        let res = self.inner.negotiate(bound, scope);
        log::info!("negotiate: {:?}", res);
        res
    }

    fn check(&self, client_id: &str, passphrase: Option<&[u8]>) -> Result<(), RegistrarError> {
        log::info!("check");
        self.inner.check(client_id, passphrase)
    }
}

struct MyAuthorizer {
    inner: AuthMap<RandomGenerator>,
}

impl Authorizer for MyAuthorizer {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        log::info!("authorize");
        self.inner.authorize(grant)
    }

    fn extract(&mut self, token: &str) -> Result<Option<Grant>, ()> {
        log::info!("extract");
        self.inner.extract(token)
    }
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

struct MySolicitor {
}

impl OwnerSolicitor<OAuthRequest> for MySolicitor {
    fn check_consent(&mut self, request: &mut OAuthRequest, pre_grant: &PreGrant) -> OwnerConsent<OAuthResponse> {
        log::info!("check_consent: {:?}, {:?}", request, pre_grant);
        OwnerConsent::InProgress(
            OAuthResponse::ok()
                .content_type("text/html")
                .unwrap()
                .body(&consent_page_html("/authorize".into(), pre_grant)),
        )
    }
}

struct AuthState {
    endpoint: Generic<MyRegistrar, MyAuthorizer, MyIssuer, MySolicitor, Vec<Scope>, fn() -> OAuthResponse>,
}

impl AuthState {
    fn new() -> Self {
        let registrar = vec![Client::public(
            "LocalClient",
            "http://localhost:8021/endpoint".parse().unwrap(),
            "default-scope".parse().unwrap(),
        )]
        .into_iter()
        .collect();
        let registrar = MyRegistrar { inner: registrar };

        let authorizer = AuthMap::new(RandomGenerator::new(16));
        let authorizer = MyAuthorizer { inner: authorizer };

        let issuer = TokenMap::new(RandomGenerator::new(16));
        let issuer = MyIssuer { inner: issuer };

        let solicitor = MySolicitor{};

        AuthState {
            endpoint: Generic {
                registrar,
                authorizer,
                issuer,
                solicitor: solicitor,
                scopes: vec!["default-scope".parse().unwrap()],
                response: OAuthResponse::ok,
            },
        }
    }
}

impl Actor for AuthState {
    type Context = Context<Self>;
}

impl<Op> Handler<OAuthMessage<Op, ()>> for AuthState
where
    Op: OAuthOperation,
{
    type Result = Result<Op::Item, Op::Error>;

    fn handle(&mut self, msg: OAuthMessage<Op, ()>, _: &mut Self::Context) -> Self::Result {
        let (op, _) = msg.into_inner();
        op.run(&mut self.endpoint)
    }
}

struct State {
    auth: Addr<AuthState>,
}

impl State {
    fn new() -> State {
        State {
            auth: AuthState::new().start(),
        }
    }
}

fn consent_page_html(route: &str, grant: &PreGrant) -> String {
    macro_rules! template {
        () => {
                "<html>'{0:}' (at {1:}) is requesting permission for '{2:}'
                <form method=\"post\">
                    <input type=\"submit\" value=\"Accept\" formaction=\"{4:}?response_type=code&client_id={3:}&allow=true\">
                    <input type=\"submit\" value=\"Deny\" formaction=\"{4:}?response_type=code&client_id={3:}\">
                </form>
                </html>"
        };
    }

    format!(
        template!(),
        grant.client_id, grant.redirect_uri, grant.scope, grant.client_id, &route
    )
}

fn get_authorization(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("get_authorization");
    Compat01As03::new(state.auth.send(Authorize(oath_req).wrap(())))
}

fn post_authorization(
    req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_authorization");
    // Some authentication should be performed here in production cases
    Compat01As03::new(
        state
            .auth
            .send(Authorize(oath_req).wrap(())),
    )
}

fn post_token(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_token");
    Compat01As03::new(state.auth.send(Token(oath_req).wrap(())))
}

fn post_refresh(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    Compat01As03::new(state.auth.send(Refresh(oath_req).wrap(())))
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
