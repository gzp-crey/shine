use super::{SignedCookieConfiguration, SignedCookieStore};
use actix_web::{dev::Payload, error::ErrorInternalServerError, Error, FromRequest, HttpRequest};
use futures::future::{err, ready, Ready};
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::{any::TypeId, cell::RefCell, collections::HashMap, marker::PhantomData, ops::Deref, rc::Rc};

#[derive(PartialEq, Clone, Debug)]
pub enum SessionStatus {
    Changed,
    Purged,
    Renewed,
    Unchanged,
}

#[derive(Debug)]
struct SessionDataInner {
    values: HashMap<String, String>,
    status: SessionStatus,
}

#[derive(Clone, Debug)]
pub struct SessionData {
    name: String,
    inner: Rc<RefCell<SessionDataInner>>,
}

impl SessionData {
    pub(crate) fn empty(name: String) -> Self {
        SessionData {
            name,
            inner: Rc::new(RefCell::new(SessionDataInner {
                values: HashMap::default(),
                status: SessionStatus::Unchanged,
            })),
        }
    }

    pub(crate) fn from_cookie(name: String, value: &str) -> Result<SessionData, ()> {
        match serde_json::from_str::<HashMap<String, String>>(value) {
            Ok(value) => Ok(SessionData {
                name,
                inner: Rc::new(RefCell::new(SessionDataInner {
                    values: value,
                    status: SessionStatus::Unchanged,
                })),
            }),
            Err(err) => {
                log::warn!("Failed to parse cooke: {}", err);
                Err(())
            }
        }
    }

    /// Get the name of the cookie
    pub fn name(&self) -> &str {
        &self.name()
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
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.values.insert(key.to_owned(), serde_json::to_string(&value)?);
        }
        Ok(())
    }

    /// Remove value from the session.
    pub fn remove(&self, key: &str) {
        let mut inner = self.inner.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.values.remove(key);
        }
    }

    /// Clear the session.
    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.values.clear()
        }
    }

    /// Removes session, both client and server side.
    pub fn purge(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.status = SessionStatus::Purged;
        inner.values.clear();
    }

    /// Renews the session key, assigning existing session values to new key.
    pub fn renew(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Renewed;
        }
    }
}

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
