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

impl Default for SessionDataInner {
    fn default() -> SessionDataInner {
        SessionDataInner {
            values: HashMap::default(),
            status: SessionStatus::Unchanged,
        }
    }
}

/// Data associated to a session and persisted using signed cookies.
#[derive(Debug)]
pub struct SessionData<C>
where
    C: 'static,
{
    name: String,
    inner: Rc<RefCell<SessionDataInner>>,
    config: Rc<C>,
}

//see https://github.com/rust-lang/rust/issues/26925
impl<C> Clone for SessionData<C>
where
    C: 'static,
{
    fn clone(&self) -> SessionData<C> {
        SessionData {
            name: self.name.clone(),
            inner: self.inner.clone(),
            config: self.config.clone(),
        }
    }
}

impl<C> SessionData<C>
where
    C: 'static,
{
    pub(crate) fn empty(name: String, config: Rc<C>) -> SessionData<C> {
        SessionData {
            name,
            inner: Rc::new(RefCell::new(Default::default())),
            config,
        }
    }

    pub(crate) fn new(name: String, values: HashMap<String, String>, config: Rc<C>) -> SessionData<C> {
        SessionData {
            name,
            inner: Rc::new(RefCell::new(SessionDataInner {
                values,
                status: SessionStatus::Unchanged,
            })),
            config,
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
        Ok(())
    }

    /// Remove value from the session.
    pub fn remove(&self, key: &str) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
        inner.values.remove(key);
    }

    /// Clear the session.
    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
        inner.values.clear();
    }

    /// Renews the session key, assigning existing session values to new key.
    pub fn renew(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Changed;
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
    pub(crate) fn into_change(self) -> Option<HashMap<String, String>> {
        let SessionDataInner { values, status } = Rc::try_unwrap(self.inner)
            .map_err(|_| ())
            .unwrap()
            .replace(Default::default());
        match status {
            SessionStatus::Unchanged => None,
            SessionStatus::Changed => Some(values),
        }
    }
}

/// Extractor to access signed cookies (session) from the request handlers. The type_id of the generic C type
/// is used to look up the sessions from the request.
pub struct Session<O: SignedCookieOptions, C: 'static>(SessionData<C>, PhantomData<O>);

impl<O, C> Deref for Session<O, C>
where
    O: SignedCookieOptions,
    C: 'static,
{
    type Target = SessionData<C>;

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
        if let Some(data) = extensions.get::<SessionData<C>>() {
            ok(Session(data.clone(), PhantomData))
        } else {
            err(ErrorInternalServerError(
                "SignedCookie is not configured, to configure use App::wrap(SignCookie::new()...)",
            ))
        }
    }
}
