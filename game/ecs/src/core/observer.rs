use std::sync::{Arc, Mutex, Weak};

pub enum ObserverResult {
    KeepObserving,
    StopObserving,
}

pub type Observer<E> = dyn 'static + Send + Sync + Fn(&E) -> ObserverResult;
type ObserverStrong<E> = Arc<Observer<E>>;
type ObserverWeak<E> = Weak<Observer<E>>;

pub struct Subscription<E>(ObserverStrong<E>);

/// Handle event dispatching to the observers
pub struct ObserveDispatcher<E>
where
    E: 'static,
{
    observers: Mutex<Vec<ObserverWeak<E>>>,
}

impl<E> ObserveDispatcher<E>
where
    E: 'static,
{
    pub fn new() -> ObserveDispatcher<E> {
        ObserveDispatcher {
            observers: Mutex::new(Vec::new()),
        }
    }

    fn subscribe_impl(&self, observer: ObserverStrong<E>) -> Subscription<E> {
        let mut observers = self.observers.lock().unwrap();
        let weak = Arc::downgrade(&observer);
        observers.push(weak);
        Subscription(observer)
    }

    /// Subscribe a new observer.
    #[must_use]
    pub fn subscribe<T>(&self, observer: T) -> Subscription<E>
    where
        T: 'static + Send + Sync + Fn(&E) -> ObserverResult,
    {
        let observer: ObserverStrong<E> = Arc::from(observer);
        self.subscribe_impl(observer)
    }

    /// Subscribe a new boxed observer.
    #[must_use]
    pub fn subscribe_boxed(&self, observer: Box<Observer<E>>) -> Subscription<E> {
        let observer = Arc::from(observer);
        self.subscribe_impl(observer)
    }

    /// Perform a strict unsubscription and remove observer from the queue.
    pub fn unsubscribe(&self, subscription: Subscription<E>) {
        let mut observers = self.observers.lock().unwrap();
        let weak = Arc::downgrade(&subscription.0);
        let len_before = observers.len();
        observers.retain(|o| !o.ptr_eq(&weak));
        if len_before == observers.len() {
            log::warn!("Observer was not subscribed");
        }
    }

    pub fn notify_all(&mut self, event: E) {
        let mut observers = self.observers.lock().unwrap();
        observers.retain(|observer| {
            if let Some(observer) = observer.upgrade() {
                match (observer)(&event) {
                    ObserverResult::KeepObserving => true,
                    ObserverResult::StopObserving => false,
                }
            } else {
                false
            }
        });
    }
}

impl<E> Default for ObserveDispatcher<E> {
    fn default() -> Self {
        Self::new()
    }
}
/*
/// Keep track of the active and requested subscriptions.
pub struct SubscriptionRequest<E> {
    request: Option<ObserverStrong<E>>,
    active: Option<Subscription<E>>,
}

impl<E> SubscriptionRequest<E> {
    /// Cancel the active subscription by dropping the reference.
    pub fn with_unsubscribe(self) -> SubscriptionRequest<E> {
        SubscriptionRequest { active: None, ..self }
    }

    /// Change requested observer.
    pub fn with_request<T>(self, observer: T) -> SubscriptionRequest<E>
    where
        T: 'static + Fn(&E) -> ObserverResult,
    {
        SubscriptionRequest {
            request: Some(Arc::from(observer)),
            ..self
        }
    }

    /// Replace the subscription.
    pub fn subscribe(&mut self, dispatcher: &ObserveDispatcher<E>) {
        if let Some(observer) = self.request.take() {
            self.active = Some(dispatcher.subscribe_impl(observer));
        }
    }
}
*/
