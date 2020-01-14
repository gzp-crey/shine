use super::{SessionData, SignedCookieMiddleware};
use actix_service::{Service, Transform};
use actix_web::{
    cookie::CookieJar,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorInternalServerError,
    Error, HttpMessage,
};
use futures::future::{ok, Ready};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

pub use actix_web::cookie::{Key, SameSite};

pub enum CookieSecurity {
    Signed(Key),
    Private(Key),
}

pub trait SignedCookieConfiguration: Any {
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

#[derive(Clone)]
pub(crate) struct SignedCookieStore(Rc<RefCell<HashMap<TypeId, SessionData>>>);

impl SignedCookieStore {
    pub(crate) fn get_session(&self, type_id: TypeId) -> Result<SessionData, Error> {
        let inner = self.0.borrow_mut();
        inner
            .get(&type_id)
            .map(|session| (*session).clone())
            .ok_or(ErrorInternalServerError(
                "Session is not configured, to configure use App::wrap(SignCookie::new().add(...))",
            ))
    }

    pub(crate) fn get_changes(&self, type_id: TypeId) -> Option<HashMap<String, String>> {
        Some(HashMap::new())
    }
}

#[derive(Clone)]
pub struct SignedCookie(Rc<HashMap<String, (TypeId, Box<dyn SignedCookieConfiguration>)>>);

impl SignedCookie {
    pub fn new() -> SignedCookie {
        SignedCookie(Rc::new(HashMap::new()))
    }

    pub fn add<C: 'static + SignedCookieConfiguration>(self, cookie: C) -> Self {
        let name = C::name().to_string();
        let mut inner = Rc::try_unwrap(self.0).map_err(|_| ()).unwrap();
        inner.insert(name, (TypeId::of::<C>(), Box::new(cookie)));
        SignedCookie(Rc::new(inner))
    }

    /// Load cookies from request and creates the SignedCookieStore
    pub(crate) fn load(&self, req: &mut ServiceRequest) -> SignedCookieStore {
        let mut store = HashMap::new();
        if let Ok(cookies) = req.cookies() {
            for cookie in cookies.iter() {
                let cookie_name = cookie.name();
                if let Some((type_id, config)) = self.0.get(cookie_name) {
                    let mut jar = CookieJar::new();
                    jar.add_original(cookie.clone());

                    let cookie_opt = match config.security() {
                        &CookieSecurity::Signed(ref key) => jar.signed(&key).get(cookie_name),
                        &CookieSecurity::Private(ref key) => jar.private(&key).get(cookie_name),
                    };

                    let data = cookie_opt
                        .and_then(|cookie| SessionData::from_cookie(cookie_name.to_string(), cookie.value()).ok())
                        .unwrap_or_else(|| SessionData::empty(cookie_name.to_string()));
                    store.insert(type_id.to_owned(), data);
                }
            }
        }

        SignedCookieStore(Rc::new(RefCell::new(store)))
    }

    /// Collect changes from the SignedCookieStore and updates the SetCookie header in the response
    pub(crate) fn store<B>(&self, res: &mut ServiceResponse<B>) {
        if let Some(store) = res.request().extensions().get::<SignedCookieStore>() {
            for (type_id, _config) in self.0.values() {
                if let Some(_data) = store.get_changes(type_id.clone()) {
                    //todo
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
