use std::sync::{Arc, Mutex, Weak};

pub trait Observable {
    type Event;

    fn notify_all(&mut self, event: Self::Event);
}

pub enum ObserveResult {
    KeepObserving,
    StopObserving,
}

pub trait Observer<E> {
    fn on_event(&self, event: &E) -> ObserveResult;
}

pub struct Subscription<E>(Option<Arc<dyn Observer<E>>>);

impl<E> Subscription<E>
{
    fn none() -> Subscription<E>{
        Subscription(None)
    }

    /// Perform a lazy unsubscription. Observer won't be notified, by queue is updated only on the next notification requests.
    pub fn cancel(&mut self) {
        let _ = self.0.take();
    }
}

/// Wrap an function to start observing with
pub struct ObserverFn<E, F>
where
    F: Fn(&E) -> ObserveResult,
{
    function: F,
    ph: std::marker::PhantomData<dyn Fn(&E)>,
}

impl<E, F> ObserverFn<E, F>
where
    F: Fn(&E) -> ObserveResult,
{
    pub fn from_fn(function: F) -> ObserverFn<E, F> {
        ObserverFn {
            function,
            ph: std::marker::PhantomData,
        }
    }
}

impl<E, F> Observer<E> for ObserverFn<E, F>
where
    F: Fn(&E) -> ObserveResult,
{
    fn on_event(&self, event: &E) -> ObserveResult {
        (self.function)(event)
    }
}

/// Thread aware handling of observers
pub struct SyncObserveDispatcher<E> {
    observers: Mutex<Vec<Weak<dyn Observer<E>>>>,
}

impl<E> SyncObserveDispatcher<E> {
    pub fn new() -> SyncObserveDispatcher<E> {
        SyncObserveDispatcher {
            observers: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe<O>(&self, observer: O) -> Subscription<E> 
    where 
        O : Observer<E>
    {
        let mut observers = self.observers.lock().unwrap();
        let weak: Weak<dyn Observer<E>> = Arc::downgrade(observer);
        if observers.iter().any(|o| o.ptr_eq(&weak)) {
            log::warn!("Observer already registered");
        } else {
            observers.push(weak);
        }
    }

    /// Perform a strict unsubscription and remove observer from the queue.
    pub fn unsubscribe(&self, subscription: Subscription<E>) {
        let mut observers = self.observers.lock().unwrap();
        let weak = Arc::downgrade(observer);
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

impl<E> Default for SyncObserveDispatcher<E> {
    fn default() -> Self {
        Self::new()
    }
}
