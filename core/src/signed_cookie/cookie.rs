use super::SignedCookieMiddleware;
use actix_service::{Service, Transform};
use actix_web::cookie::{Cookie, CookieJar};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::HttpMessage;
use futures::future::{ok, Ready};
use log;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub use actix_web::cookie::{Key, SameSite};

pub enum CookieSecurity {
    Signed(Key),
    Private(Key),
}

pub trait SignedCookieConfiguration {
    /// The `name` of the cookie being.
    fn name() -> &'static str
    where
        Self: Sized;

    /// If set SetCookie header is not generated.
    fn read_only(&self) -> bool;

    /// The cookie jar secority, private or secure
    fn security(&self) -> &CookieSecurity;

    /// The `path` field in the session cookie being built.
    fn path(&self) -> &str;

    /// The `domain` field in the session cookie being built.
    fn domain(&self) -> &str;

    /// The `secure` field in the session cookie being built.
    ///
    /// If the `secure` is set, a cookie will only be transmitted when the
    /// connection is secure - i.e. `https`
    fn secure(&self) -> bool;

    /// The `http_only` field in the session cookie being built.
    fn http_only(&self) -> bool;

    /// The `same_site` field in the session cookie being built.
    fn same_site(&self) -> Option<SameSite>;

    /// The `max-age` field in the session cookie being built.
    fn max_age(&self) -> Option<i64>;
}

type SignedCookieInner = Rc<RefCell<HashMap<String, Box<dyn SignedCookieConfiguration>>>>;

#[derive(Clone)]
pub struct SignedCookie(SignedCookieInner);

impl SignedCookie {
    pub fn new() -> SignedCookie {
        SignedCookie(Rc::new(RefCell::new(HashMap::new())))
    }

    pub fn add<C: 'static + SignedCookieConfiguration>(&mut self, cookie: C) -> &mut Self {
        {
            let mut map = self.0.borrow_mut();
            let name = C::name().to_string();
            map.insert(name, Box::new(cookie));
        }
        self
    }

    pub fn load(&self, req: &ServiceRequest) {
        if let Ok(cookies) = req.cookies() {
            let inner = self.0.borrow_mut();
            for cookie in cookies.iter() {
                let name = cookie.name();
                if let Some(config) = inner.get(name) {
                    let mut jar = CookieJar::new();
                    jar.add_original(cookie.clone());

                    let cookie_opt = match config.security() {
                        &CookieSecurity::Signed(ref key) => jar.signed(&key).get(name),
                        &CookieSecurity::Private(ref key) => jar.private(&key).get(name),
                    };

                    if let Some(cookie) = cookie_opt {
                        if let Ok(val) = serde_json::from_str::<HashMap<String, String>>(cookie.value()) {
                            log::info!("signed cookie {}: {:?}", name, val);
                        }
                    }
                }
            }
        }
    }
}

impl<S, B: 'static> Transform<S> for SignedCookie
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>>,
    S::Future: 'static,
    S::Error: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type InitError = ();
    type Transform = SignedCookieMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SignedCookieMiddleware {
            service,
            inner: self.clone(),
        })
    }
}
