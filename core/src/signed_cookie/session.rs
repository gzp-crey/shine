use super::SignedCookieConfiguration;
use actix_web::dev::{Extensions, Payload, ServiceRequest, ServiceResponse};
use actix_web::{Error, FromRequest, HttpMessage, HttpRequest};
use futures::future::{ok, Ready};
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(PartialEq, Clone, Debug)]
pub enum SessionStatus {
    Changed,
    Purged,
    Renewed,
    Unchanged,
}

impl Default for SessionStatus {
    fn default() -> SessionStatus {
        SessionStatus::Unchanged
    }
}

#[derive(Default)]
struct SessionInner {
    state: HashMap<String, String>,
    status: SessionStatus,
}

pub struct Session<C: 'static + SignedCookieConfiguration>(Rc<RefCell<SessionInner>>, PhantomData<C>);

impl<C> Session<C>
where
    C: 'static + SignedCookieConfiguration,
{
    /// Get a `value` from the session.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        if let Some(s) = self.0.borrow().state.get(key) {
            Ok(Some(serde_json::from_str(s)?))
        } else {
            Ok(None)
        }
    }

    /// Set a `value` from the session.
    pub fn set<T: Serialize>(&self, key: &str, value: T) -> Result<(), Error> {
        let mut inner = self.0.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.state.insert(key.to_owned(), serde_json::to_string(&value)?);
        }
        Ok(())
    }

    /// Remove value from the session.
    pub fn remove(&self, key: &str) {
        let mut inner = self.0.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.state.remove(key);
        }
    }

    /// Clear the session.
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.state.clear()
        }
    }

    /// Removes session, both client and server side.
    pub fn purge(&self) {
        let mut inner = self.0.borrow_mut();
        inner.status = SessionStatus::Purged;
        inner.state.clear();
    }

    /// Renews the session key, assigning existing session state to new key.
    pub fn renew(&self) {
        let mut inner = self.0.borrow_mut();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Renewed;
        }
    }

    /*
    pub fn set_session<C2>(data: impl Iterator<Item = (String, String)>, req: &mut ServiceRequest)
    where
        C2: 'static + SignedCookieConfiguration,
    {
        let session : Session<C2> = Session::get_session(&mut *req.extensions_mut());
        let mut inner = session.0.borrow_mut();
        inner.state.extend(data);
    }

    pub fn get_changes<B, C2>(res: &mut ServiceResponse<B>) -> (SessionStatus, Option<impl Iterator<Item = (String, String)>>)
    where
        C2: 'static + SignedCookieConfiguration,
    {
        if let Some(s_impl) = res.request().extensions().get::<Session<C2>>() {
            let state = std::mem::replace(&mut s_impl.0.borrow_mut().state, HashMap::new());
            (s_impl.0.borrow().status.clone(), Some(state.into_iter()))
        } else {
            (SessionStatus::Unchanged, None)
        }
    }

    fn get_session<C2>(extensions: &mut Extensions) -> Session<C2>
    where
        C2: 'static + SignedCookieConfiguration,
    {
        if let Some(s_impl) = extensions.get::<Rc<RefCell<SessionInner>>>() {
            return Session(Rc::clone(&s_impl), PhantomData);
        }
        let inner = Rc::new(RefCell::new(SessionInner::default()));
        extensions.insert(inner.clone());
        Session(inner, PhantomData)
    }*/
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
        unimplemented!()
        //ok(Session::get_session::<C>(&mut *req.extensions_mut()))
    }
}
