use super::{SessionData, SignedCookieError, SignedCookieMiddleware};
use actix_service::{Service, Transform};
use actix_web::{
    cookie::{Cookie, CookieJar},
    dev::{ServiceRequest, ServiceResponse},
    http::{header::SET_COOKIE, HeaderValue},
    Error, HttpMessage,
};
use futures::future::{ok, Ready};
use std::{collections::HashMap, rc::Rc};

pub use actix_web::cookie::{Key, SameSite};

pub enum CookieSecurity {
    Signed(Key),
    Private(Key),
}

/// Configure a signed cookie (session). To allow easy access from request handler
/// the type of implementing structure is used. See Session.
pub trait SignedCookieOptions: 'static {
    /// The `name` of the cookie being.
    fn name(&self) -> &str;

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

/// Manage the signed cookies
//#[derive(Clone)] see https://github.com/rust-lang/rust/issues/26925
pub struct SignedCookie<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    options: Rc<O>,
    config: Rc<C>,
}

impl<O, C> Clone for SignedCookie<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    fn clone(&self) -> SignedCookie<O, C> {
        SignedCookie {
            options: self.options.clone(),
            config: self.config.clone(),
        }
    }
}

impl<O, C> SignedCookie<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    fn load_cookie(
        cookie: &Cookie<'static>,
        options: &O,
        config: Rc<C>,
    ) -> Result<SessionData<O, C>, SignedCookieError> {
        let name = cookie.name();
        let mut jar = CookieJar::new();
        jar.add_original(cookie.clone());

        let cookie = match options.security() {
            &CookieSecurity::Signed(ref key) => jar.signed(&key).get(name),
            &CookieSecurity::Private(ref key) => jar.private(&key).get(name),
        }
        .ok_or(SignedCookieError::Verification)?;

        let values = serde_json::from_str::<HashMap<String, String>>(cookie.value())?;
        Ok(SessionData::new(name.to_owned(), values, config))
    }

    fn set_cookie_options(cookie: &mut Cookie, options: &O) {
        cookie.set_path(options.path().to_owned());
        cookie.set_secure(options.secure());
        cookie.set_http_only(options.http_only());

        if let Some(domain) = options.domain() {
            cookie.set_domain(domain.to_owned());
        }

        if let Some(same_site) = options.same_site() {
            cookie.set_same_site(same_site);
        }
    }

    fn set_cookie<B>(res: &mut ServiceResponse<B>, options: &O, values: HashMap<String, String>) -> Result<(), Error> {
        let value = serde_json::to_string(&values).map_err(SignedCookieError::Serialize)?;

        let mut cookie = Cookie::new(options.name().to_owned(), value);
        Self::set_cookie_options(&mut cookie, options);

        if let Some(max_age) = options.max_age() {
            cookie.set_max_age(max_age);
        }

        let mut jar = CookieJar::new();
        match options.security() {
            CookieSecurity::Signed(key) => jar.signed(key).add(cookie),
            CookieSecurity::Private(key) => jar.private(key).add(cookie),
        }

        for cookie in jar.delta() {
            let val = HeaderValue::from_str(&cookie.encoded().to_string())?;
            res.headers_mut().append(SET_COOKIE, val);
        }

        Ok(())
    }

    fn purge_cookie<B>(res: &mut ServiceResponse<B>, options: &O) -> Result<(), Error> {
        let mut cookie = Cookie::new(options.name().to_owned(), "");
        Self::set_cookie_options(&mut cookie, options);

        //removed as postman does not supports cookie.set_max_age(time::Duration::seconds(0));
        cookie.set_expires(time::now_utc() - time::Duration::days(365));

        let val = HeaderValue::from_str(&cookie.to_string())?;
        res.headers_mut().append(SET_COOKIE, val);

        Ok(())
    }
}

impl<O, C> SignedCookie<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    pub fn new(options: O, config: C) -> Self {
        SignedCookie {
            options: Rc::new(options),
            config: Rc::new(config),
        }
    }

    /// Load cookies from request and creates the SignedCookieStore
    pub(crate) fn load(&self, req: &mut ServiceRequest) -> SessionData<O, C> {
        let name = self.options.name().to_owned();
        let config = self.config.clone();
        if let Ok(cookies) = req.cookies() {
            if let Some(cookie) = cookies.iter().find(|x| x.name() == name) {
                log::debug!("Loading cookie {}: {:?}", name, cookie);
                return Self::load_cookie(cookie, &*self.options, self.config.clone()).unwrap_or_else(|err| {
                    log::warn!("Failed to parse cookie: {:?}", err);
                    SessionData::empty(name, config)
                });
            }
        }
        SessionData::empty(name, config)
    }

    /// Collect changes from the SignedCookieStore and updates the SetCookie header in the response
    pub(crate) fn store<B>(&self, data: SessionData<O, C>, res: &mut ServiceResponse<B>) {
        if let Some(changes) = data.into_change() {
            if !self.options.read_only() {
                if changes.is_empty() {
                    log::info!("Purge cookie [{}]", self.options.name());
                    Self::purge_cookie(res, &*self.options)
                        .unwrap_or_else(|err| log::warn!("Failed to purge cookie: {:?}", err));
                } else {
                    log::info!("Set cookie [{}]", self.options.name());
                    log::debug!("Cookie values [{}]: {:?}", self.options.name(), changes);
                    Self::set_cookie(res, &*self.options, changes)
                        .unwrap_or_else(|err| log::warn!("Failed to set cookie: {:?}", err));
                }
            } else {
                log::warn!("Read only cookie [{}] changed", self.options.name());
            }
        }
    }
}

impl<O, C, S, B: 'static> Transform<S> for SignedCookie<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>>,
    S::Future: 'static,
    S::Error: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type InitError = ();
    type Transform = SignedCookieMiddleware<O, C, S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SignedCookieMiddleware {
            service,
            inner: self.clone(),
        })
    }
}
