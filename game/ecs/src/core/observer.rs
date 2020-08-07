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

impl<T, E> Observer<E> for T
where
    T: Fn(&E) -> ObserveResult,
{
    fn on_event(&self, event: &E) -> ObserveResult {
        (self)(event)
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

    pub fn subscribe(&self, observer: &Arc<dyn Observer<E>>) {
        let mut observers = self.observers.lock().unwrap();
        let weak: Weak<dyn Observer<E>> = Arc::downgrade(observer);
        if observers.iter().any(|o| o.ptr_eq(&weak)) {
            log::warn!("Observer already registered");
        } else {
            observers.push(weak);
        }
    }

    pub fn unsubscribe(&self, observer: &Arc<dyn Observer<E>>) {
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
