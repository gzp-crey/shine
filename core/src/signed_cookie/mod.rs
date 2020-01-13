use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use actix_web::dev::{Extensions, Payload, RequestHead, ServiceRequest, ServiceResponse};
use actix_web::{Error, FromRequest, HttpMessage, HttpRequest};
use futures::future::{ok, Ready};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

mod cookie;
pub use self::cookie::CookieSession;

pub struct Session(Rc<RefCell<SessionInner>>);

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
    pub status: SessionStatus,
}

impl Session {
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

    pub fn set_session(data: impl Iterator<Item = (String, String)>, req: &mut ServiceRequest) {
        let session = Session::get_session(&mut *req.extensions_mut());
        let mut inner = session.0.borrow_mut();
        inner.state.extend(data);
    }

    pub fn get_changes<B>(res: &mut ServiceResponse<B>) -> (SessionStatus, Option<impl Iterator<Item = (String, String)>>) {
        if let Some(s_impl) = res.request().extensions().get::<Rc<RefCell<SessionInner>>>() {
            let state = std::mem::replace(&mut s_impl.borrow_mut().state, HashMap::new());
            (s_impl.borrow().status.clone(), Some(state.into_iter()))
        } else {
            (SessionStatus::Unchanged, None)
        }
    }

    fn get_session(extensions: &mut Extensions) -> Session {
        if let Some(s_impl) = extensions.get::<Rc<RefCell<SessionInner>>>() {
            return Session(Rc::clone(&s_impl));
        }
        let inner = Rc::new(RefCell::new(SessionInner::default()));
        extensions.insert(inner.clone());
        Session(inner)
    }
}

/// Extractor implementation for Session type.
///
/// ```rust
/// # use actix_web::*;
/// use actix_session::Session;
///
/// fn index(session: Session) -> Result<&'static str> {
///     // access session data
///     if let Some(count) = session.get::<i32>("counter")? {
///         session.set("counter", count + 1)?;
///     } else {
///         session.set("counter", 1)?;
///     }
///
///     Ok("Welcome!")
/// }
/// # fn main() {}
/// ```
impl FromRequest for Session {
    type Error = Error;
    type Future = Ready<Result<Session, Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Session::get_session(&mut *req.extensions_mut()))
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{test, HttpResponse};

    use super::*;

    #[test]
    fn session() {
        let mut req = test::TestRequest::default().to_srv_request();

        Session::set_session(vec![("key".to_string(), "\"value\"".to_string())].into_iter(), &mut req);
        let session = Session::get_session(&mut *req.extensions_mut());
        let res = session.get::<String>("key").unwrap();
        assert_eq!(res, Some("value".to_string()));

        session.set("key2", "value2".to_string()).unwrap();
        session.remove("key");

        let mut res = req.into_response(HttpResponse::Ok().finish());
        let (_status, state) = Session::get_changes(&mut res);
        let changes: Vec<_> = state.unwrap().collect();
        assert_eq!(changes, [("key2".to_string(), "\"value2\"".to_string())]);
    }

    #[test]
    fn get_session() {
        let mut req = test::TestRequest::default().to_srv_request();

        Session::set_session(vec![("key".to_string(), "\"value\"".to_string())].into_iter(), &mut req);

        let session = req.get_session();
        let res = session.get::<String>("key").unwrap();
        assert_eq!(res, Some("value".to_string()));
    }

    #[test]
    fn get_session_from_request_head() {
        let mut req = test::TestRequest::default().to_srv_request();

        Session::set_session(vec![("key".to_string(), "\"value\"".to_string())].into_iter(), &mut req);

        let session = req.head_mut().get_session();
        let res = session.get::<String>("key").unwrap();
        assert_eq!(res, Some("value".to_string()));
    }

    #[test]
    fn purge_session() {
        let req = test::TestRequest::default().to_srv_request();
        let session = Session::get_session(&mut *req.extensions_mut());
        assert_eq!(session.0.borrow().status, SessionStatus::Unchanged);
        session.purge();
        assert_eq!(session.0.borrow().status, SessionStatus::Purged);
    }

    #[test]
    fn renew_session() {
        let req = test::TestRequest::default().to_srv_request();
        let session = Session::get_session(&mut *req.extensions_mut());
        assert_eq!(session.0.borrow().status, SessionStatus::Unchanged);
        session.renew();
        assert_eq!(session.0.borrow().status, SessionStatus::Renewed);
    }
}
