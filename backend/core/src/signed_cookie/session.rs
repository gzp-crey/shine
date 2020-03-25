use super::SignedCookieOptions;
use actix_web::{dev::Payload, error::ErrorInternalServerError, Error, FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, ops::Deref, rc::Rc};

#[derive(PartialEq, Clone, Debug)]
enum SessionStatus {
    Changed,
    Unchanged,
}

#[derive(Debug)]
struct SessionDataInner {
    values: HashMap<String, String>,
    status: SessionStatus,
}

/// Data associated to a session and persisted using signed cookies.
#[derive(Debug)]
pub struct SessionData<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    name: String,
    inner: Rc<RefCell<SessionDataInner>>,
    config: Rc<C>,
    _options: PhantomData<O>,
}

//see https://github.com/rust-lang/rust/issues/26925
impl<O, C> Clone for SessionData<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    fn clone(&self) -> SessionData<O, C> {
        SessionData {
            name: self.name.clone(),
            inner: self.inner.clone(),
            config: self.config.clone(),
            _options: self._options.clone(),
        }
    }
}

impl<O, C> SessionData<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    pub(crate) fn empty(name: String, config: Rc<C>) -> SessionData<O, C> {
        log::debug!("Init [{}] cookie data to empty", name);
        SessionData {
            name,
            inner: Rc::new(RefCell::new(SessionDataInner {
                values: HashMap::default(),
                status: SessionStatus::Unchanged,
            })),
            config,
            _options: PhantomData,
        }
    }

    pub(crate) fn new(name: String, values: HashMap<String, String>, config: Rc<C>) -> SessionData<O, C> {
        log::debug!("Init [{}] cookie data with {:?}", name, values);
        SessionData {
            name,
            inner: Rc::new(RefCell::new(SessionDataInner {
                values,
                status: SessionStatus::Unchanged,
            })),
            config,
            _options: PhantomData,
        }
    }

    /// Get the name of the cookie
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn config(&self) -> Rc<C> {
        self.config.clone()
    }

    /// Get a `value` from the session.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        if let Some(s) = self.inner.borrow().values.get(key) {
            Ok(Some(serde_json::from_str(s)?))
        } else {
            Ok(None)
        }
    }

    /// Set a `value` from the session.
    pub fn set<T: Serialize>(&self, key: &str, value: T) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
        inner.values.insert(key.to_owned(), serde_json::to_string(&value)?);
        log::debug!(
            "Change [{}] cookie, set [{}] => {:?} ({:?})",
            self.name,
            key,
            inner.status,
            inner.values
        );
        Ok(())
    }

    /// Remove value from the session.
    pub fn remove(&self, key: &str) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
        inner.values.remove(key);
        log::debug!(
            "Change [{}] cookie, remove [{}] => {:?} ({:?})",
            self.name,
            key,
            inner.status,
            inner.values
        );
    }

    /// Clear the session.
    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
        inner.values.clear();
        log::debug!(
            "Change [{}] cookie, clear => {:?} ({:?})",
            self.name,
            inner.status,
            inner.values
        );
    }

    /// Renews the session key, assigning existing session values to new key.
    pub fn renew(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
        log::debug!(
            "Change [{}] cookie, renew => {:?} ({:?})",
            self.name,
            inner.status,
            inner.values
        );
    }

    pub fn is_changed(&self) -> bool {
        match &self.inner.borrow().status {
            SessionStatus::Unchanged => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().values.is_empty()
    }

    /// Return the changes in cookie state, or None if session requires no update
    pub(crate) fn into_change(&self) -> Option<HashMap<String, String>> {
        let SessionDataInner { values, status } = self.inner.replace(SessionDataInner {
            values: HashMap::default(),
            status: SessionStatus::Unchanged,
        });
        log::trace!("Cookie values for {}: {:?} ({:?})", self.name, status, values);
        match status {
            SessionStatus::Unchanged => None,
            SessionStatus::Changed => Some(values),
        }
    }
}

/// Extractor to access signed cookies (session) from the request handlers. The type_id of the generic C type
/// is used to look up the sessions from the request.
pub struct Session<O: SignedCookieOptions, C: 'static>(SessionData<O, C>, PhantomData<O>);

impl<O, C> Deref for Session<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    type Target = SessionData<O, C>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<O, C> FromRequest for Session<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let extensions = req.extensions();
        if let Some(data) = extensions.get::<SessionData<O, C>>() {
            ok(Session(data.clone(), PhantomData))
        } else {
            err(ErrorInternalServerError(
                "SignedCookie is not configured, to configure use App::wrap(SignedCookie::new(...))",
            ))
        }
    }
}
