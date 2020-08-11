use std::{
    mem,
    sync::{Arc, Mutex, Weak},
};

pub enum ObserveResult {
    KeepObserving,
    StopObserving,
}

pub trait Observer<E>: 'static + Send + Sync {
    fn on_event(&self, event: &E) -> ObserveResult;
}

pub struct Subscription<E>(Arc<dyn Observer<E>>);

impl<E, T> Observer<E> for T
where
    T: 'static + Send + Sync + Fn(&E) -> ObserveResult,
{
    fn on_event(&self, event: &E) -> ObserveResult {
        (self)(event)
    }
}
/// Thread aware handling of observers
pub struct ObserveDispatcher<E>
where
    E: 'static,
{
    observers: Mutex<Vec<Weak<dyn Observer<E>>>>,
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

    #[must_use]
    fn subscribe_arc(&self, observer: Arc<dyn Observer<E>>) -> Subscription<E> {
        let mut observers = self.observers.lock().unwrap();
        let weak: Weak<dyn Observer<E>> = Arc::downgrade(&observer);
        if observers.iter().any(|o| o.ptr_eq(&weak)) {
            log::warn!("Observer already registered");
        } else {
            observers.push(weak);
        }
        Subscription(observer)
    }

    #[must_use]
    pub fn subscribe<O>(&self, observer: O) -> Subscription<E>
    where
        O: 'static + Observer<E>,
    {
        let observer: Arc<dyn Observer<E>> = Arc::new(observer);
        self.subscribe_arc(observer)
    }

    #[must_use]
    pub fn subscribe_boxed(&self, observer: Box<dyn Observer<E>>) -> Subscription<E> {
        let observer: Arc<dyn Observer<E>> = Arc::from(observer);
        self.subscribe_arc(observer)
    }

    #[must_use]
    pub fn subscribe_fn<T>(&self, observer: T) -> Subscription<E>
    where
        T: 'static + Send + Sync + Fn(&E) -> ObserveResult,
    {
        let observer: Arc<dyn Observer<E>> = Arc::from(observer);
        self.subscribe_arc(observer)
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
                match observer.on_event(&event) {
                    ObserveResult::KeepObserving => true,
                    ObserveResult::StopObserving => false,
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

/// Keep track of the active and requested subscriptions.
pub enum SubscriptionRequest<E> {
    /// No subscription.
    None,
    /// Request a new subscription.
    Request(Box<dyn Observer<E>>),
    /// Keep the subscription alive.
    Active(Subscription<E>),
}

impl<E> SubscriptionRequest<E> {
    /// Cancel the active subscription by dropping the reference.
    pub fn with_cancel_active(self) -> SubscriptionRequest<E> {
        if let SubscriptionRequest::Request(observer) = self {
            SubscriptionRequest::Request(observer)
        } else {
            SubscriptionRequest::None
        }
    }

    /// Replace subscription in the shader.
    pub fn subscribe(&mut self, dispatcher: &ObserveDispatcher<E>) {
        *self = match mem::replace(self, SubscriptionRequest::None) {
            SubscriptionRequest::Request(observer) => SubscriptionRequest::Active(dispatcher.subscribe_boxed(observer)),
            sub => sub,
        };
    }
}
