use actix::{Actor, Addr, Context, Handler, MailboxError};
use actix_web::{web, HttpRequest};
use futures::compat::{Compat, Compat01As03};
use futures::{Future, TryFuture};
use oxide_auth::{
    endpoint::{Endpoint, OwnerConsent, OwnerSolicitor, PreGrant},
    frontends::simple::endpoint::{ErrorInto, FnSolicitor, Generic, Vacant},
    primitives::prelude::{AuthMap, Client, ClientMap, ClientUrl, RandomGenerator, Scope, TokenMap},
    primitives::registrar::{BoundClient, Registrar, RegistrarError},
};
use oxide_auth_actix::{Authorize, OAuthMessage, OAuthOperation, OAuthRequest, OAuthResponse, Refresh, Token, WebError};

enum AuthExtras {
    AuthGet,
    AuthPost(String),
    Nothing,
}

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

struct AuthState {
    endpoint:
        Generic<MyRegistrar, AuthMap<RandomGenerator>, TokenMap<RandomGenerator>, Vacant, Vec<Scope>, fn() -> OAuthResponse>,
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

        AuthState {
            endpoint: Generic {
                registrar,
                // Authorization tokens are 16 byte random keys to a memory hash map.
                authorizer: AuthMap::new(RandomGenerator::new(16)),
                // Bearer tokens are also random generated but 256-bit tokens, since they live longer
                // and this example is somewhat paranoid.
                //
                // We could also use a `TokenSigner::ephemeral` here to create signed tokens which can
                // be read and parsed by anyone, but not maliciously created. However, they can not be
                // revoked and thus don't offer even longer lived refresh tokens.
                issuer: TokenMap::new(RandomGenerator::new(16)),

                solicitor: Vacant,

                // A single scope that will guard resources for this endpoint
                scopes: vec!["default-scope".parse().unwrap()],

                response: OAuthResponse::ok,
            },
        }
    }

    pub fn with_solicitor<'a, S>(&'a mut self, solicitor: S) -> impl Endpoint<OAuthRequest, Error = WebError> + 'a
    where
        S: OwnerSolicitor<OAuthRequest> + 'static,
    {
        ErrorInto::new(Generic {
            authorizer: &mut self.endpoint.authorizer,
            registrar: &mut self.endpoint.registrar,
            issuer: &mut self.endpoint.issuer,
            solicitor,
            scopes: &mut self.endpoint.scopes,
            response: OAuthResponse::ok,
        })
    }
}

impl Actor for AuthState {
    type Context = Context<Self>;
}

impl<Op> Handler<OAuthMessage<Op, AuthExtras>> for AuthState
where
    Op: OAuthOperation,
{
    type Result = Result<Op::Item, Op::Error>;

    fn handle(&mut self, msg: OAuthMessage<Op, AuthExtras>, _: &mut Self::Context) -> Self::Result {
        let (op, ex) = msg.into_inner();

        match ex {
            AuthExtras::AuthGet => {
                let solicitor = FnSolicitor(move |_: &mut OAuthRequest, pre_grant: &PreGrant| {
                    // This will display a page to the user asking for his permission to proceed. The submitted form
                    // will then trigger the other authorization handler which actually completes the flow.
                    OwnerConsent::InProgress(
                        OAuthResponse::ok()
                            .content_type("text/html")
                            .unwrap()
                            .body(&consent_page_html("/authorize".into(), pre_grant)),
                    )
                });

                op.run(self.with_solicitor(solicitor))
            }
            AuthExtras::AuthPost(query_string) => {
                let solicitor = FnSolicitor(move |_: &mut OAuthRequest, _: &PreGrant| {
                    if query_string.contains("allow") {
                        OwnerConsent::Authorized("dummy user".to_owned())
                    } else {
                        OwnerConsent::Denied
                    }
                });

                op.run(self.with_solicitor(solicitor))
            }
            _ => op.run(&mut self.endpoint),
        }
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
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    log::info!("get_authorization");
    Compat01As03::new(state.auth.send(Authorize(oath_req).wrap(AuthExtras::AuthGet)))
}

fn post_authorization(
    req: HttpRequest,
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_authorization");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    // Some authentication should be performed here in production cases
    Compat01As03::new(
        state
            .auth
            .send(Authorize(oath_req).wrap(AuthExtras::AuthPost(req.query_string().to_owned()))),
    )
}

fn post_token(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    log::info!("post_token");
    Compat01As03::new(state.auth.send(Token(oath_req).wrap(AuthExtras::Nothing)))
}

fn post_refresh(
    oath_req: OAuthRequest,
    state: web::Data<State>,
) -> impl TryFuture<Ok = Result<OAuthResponse, WebError>, Error = MailboxError> {
    Compat01As03::new(state.auth.send(Refresh(oath_req).wrap(AuthExtras::Nothing)))
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
