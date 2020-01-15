use super::{SignedCookieConfiguration, SignedCookieStore};
use actix_web::{dev::Payload, error::ErrorInternalServerError, Error, FromRequest, HttpRequest};
use futures::future::{err, ready, Ready};
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::{any::TypeId, cell::RefCell, collections::HashMap, marker::PhantomData, ops::Deref, rc::Rc};

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
#[derive(Clone, Debug)]
pub struct SessionData {
    name: String,
    inner: Rc<RefCell<SessionDataInner>>,
}

impl SessionData {
    pub(crate) fn empty(name: String) -> Self {
        SessionData {
            name,
            inner: Rc::new(RefCell::new(Default::default())),
        }
    }

    pub(crate) fn new(name: String, values: HashMap<String, String>) -> SessionData {
        SessionData {
            name,
            inner: Rc::new(RefCell::new(SessionDataInner {
                values,
                status: SessionStatus::Unchanged,
            })),
        }
    }

    /// Get the name of the cookie
    pub fn name(&self) -> &str {
        &self.name
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
pub struct Session<C: 'static + SignedCookieConfiguration>(SessionData, PhantomData<C>);

impl<C> Deref for Session<C>
where
    C: 'static + SignedCookieConfiguration,
{
    type Target = SessionData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C> FromRequest for Session<C>
where
    C: 'static + SignedCookieConfiguration,
{
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let extensions = req.extensions();
        if let Some(signed_cookie) = extensions.get::<SignedCookieStore>() {
            ready(
                signed_cookie
                    .get_session(TypeId::of::<C>())
                    .map(|data| Session(data, PhantomData)),
            )
        } else {
            err(ErrorInternalServerError(
                "SignedCookie is not configured, to configure use App::wrap(SignCookie::new()...)",
            ))
        }
    }
}
