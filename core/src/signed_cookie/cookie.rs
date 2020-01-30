use super::{SessionData, SignedCookieError, SignedCookieMiddleware};
use actix_service::{Service, Transform};
use actix_web::{
    cookie::{Cookie, CookieJar},
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorInternalServerError,
    http::{header::SET_COOKIE, HeaderValue},
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

/// Configure a signed cookie (session). To allow easy access from request handler
/// the type of implementing structure is used. See Session.
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
    fn path(&self) -> &str {
        "/"
    }

    /// The `domain` field in the session cookie being built.
    fn domain(&self) -> Option<&str> {
        None
    }

    /// The `secure` field in the session cookie being built.
    ///
    /// If the `secure` is set, a cookie will only be transmitted when the
    /// connection is secure - i.e. `https`
    fn secure(&self) -> bool {
        true
    }

    /// The `http_only` field in the session cookie being built.
    ///
    /// If 'http_only' is set, a cookie cannot be accessed by client-side APIs, such as JavaScript.
    fn http_only(&self) -> bool {
        true
    }

    /// The `same_site` field in the session cookie being built.
    fn same_site(&self) -> Option<SameSite> {
        None
    }

    /// The `max-age` field in the session cookie being built.
    fn max_age(&self) -> Option<time::Duration> {
        None
    }
}

/// Store the signed cookies for the request-respond handling
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
        let mut inner = self.0.borrow_mut();
        inner.remove(&type_id).and_then(|session| session.into_change())
    }
}

/// Manage the signed cookies
#[derive(Clone)]
pub struct SignedCookie(Rc<HashMap<String, (TypeId, Box<dyn SignedCookieConfiguration>)>>);

impl SignedCookie {
    fn load_cookie(cookie: &Cookie<'static>, config: &dyn SignedCookieConfiguration) -> Result<SessionData, SignedCookieError> {
        let name = cookie.name();
        let mut jar = CookieJar::new();
        jar.add_original(cookie.clone());

        let cookie = match config.security() {
            &CookieSecurity::Signed(ref key) => jar.signed(&key).get(name),
            &CookieSecurity::Private(ref key) => jar.private(&key).get(name),
        }
        .ok_or(SignedCookieError::Verification)?;

        let values = serde_json::from_str::<HashMap<String, String>>(cookie.value())?;
        Ok(SessionData::new(name.to_owned(), values))
    }

    fn set_cookie_options(cookie: &mut Cookie, config: &dyn SignedCookieConfiguration) {
        cookie.set_path(config.path().to_owned());
        cookie.set_secure(config.secure());
        cookie.set_http_only(config.http_only());

        if let Some(domain) = config.domain() {
            cookie.set_domain(domain.to_owned());
        }

        if let Some(same_site) = config.same_site() {
            cookie.set_same_site(same_site);
        }
    }

    fn set_cookie<B>(
        res: &mut ServiceResponse<B>,
        name: &str,
        config: &dyn SignedCookieConfiguration,
        values: HashMap<String, String>,
    ) -> Result<(), Error> {
        let value = serde_json::to_string(&values).map_err(SignedCookieError::Serialize)?;

        let mut cookie = Cookie::new(name.to_owned(), value);
        Self::set_cookie_options(&mut cookie, config);

        if let Some(max_age) = config.max_age() {
            cookie.set_max_age(max_age);
        }

        let mut jar = CookieJar::new();
        match config.security() {
            CookieSecurity::Signed(key) => jar.signed(key).add(cookie),
            CookieSecurity::Private(key) => jar.private(key).add(cookie),
        }

        for cookie in jar.delta() {
            let val = HeaderValue::from_str(&cookie.encoded().to_string())?;
            res.headers_mut().append(SET_COOKIE, val);
        }

        Ok(())
    }

    fn purge_cookie<B>(res: &mut ServiceResponse<B>, name: &str, config: &dyn SignedCookieConfiguration) -> Result<(), Error> {
        let mut cookie = Cookie::new(name.to_owned(), "");
        Self::set_cookie_options(&mut cookie, config);

        //removed as postman does not supports cookie.set_max_age(time::Duration::seconds(0));
        cookie.set_expires(time::now_utc() - time::Duration::days(365));

        let val = HeaderValue::from_str(&cookie.to_string())?;
        res.headers_mut().append(SET_COOKIE, val);

        Ok(())
    }
}

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
        for (name, (type_id, config)) in self.0.iter() {
            let mut data = SessionData::empty(name.to_owned());
            if let Ok(cookies) = req.cookies() {
                log::trace!("cookies: {:?}", cookies);
                if let Some(cookie) = cookies.iter().find(|x| x.name() == name) {
                    log::trace!("cookie {}: {:?}", name, cookie);
                    match Self::load_cookie(cookie, &**config) {
                        Ok(d) => data = d,
                        Err(err) => log::warn!("Failed to parse cookie: {:?}", err),
                    };
                }
            }
            if !data.is_empty() {
                log::info!("Loaded cookie {}: {:?}", name, data);
            } else {
                log::debug!("Missing cookie {}", name);
            }
            store.insert(type_id.to_owned(), data);
        }

        SignedCookieStore(Rc::new(RefCell::new(store)))
    }

    /// Collect changes from the SignedCookieStore and updates the SetCookie header in the response
    pub(crate) fn store<B>(&self, store: SignedCookieStore, res: &mut ServiceResponse<B>) {
        for (name, (type_id, config)) in self.0.iter() {
            if let Some(data) = store.get_changes(type_id.clone()) {
                if !config.read_only() {
                    if data.is_empty() {
                        log::debug!("Purge cookie {}", name);
                        Self::purge_cookie(res, name, &**config)
                            .unwrap_or_else(|err| log::warn!("Failed to purge cookie: {:?}", err));
                    } else {
                        log::debug!("Set cookie {}", name);
                        Self::set_cookie(res, name, &**config, data)
                            .unwrap_or_else(|err| log::warn!("Failed to set cookie: {:?}", err));
                    }
                } else {
                    log::warn!("Read only cookie {} changed", name);
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
