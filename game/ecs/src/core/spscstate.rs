use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

type AtomicFlag = AtomicUsize;

/// Struct to force the alignment of the stored data match the typical size of a cache-line
/// to avoid false sharing.
#[repr(align(64))]
struct AlignedData<T>(T);

/// Triple buffer that uses atomic operations to rotate the 3 buffers during consume/produce operations
struct TripleBuffer<T> {
    buffers: UnsafeCell<[AlignedData<T>; 3]>,

    // flag bits:
    // newWrite   = (flags & 0x40)
    // produceIndex = (flags & 0x30) >> 4       buffer to be produced, write to
    // intermediateIndex = (flags & 0xC) >> 2   intermediate buffer (transit zone)
    // consumeIndex  = (flags & 0x3)            buffer to consume, consume from
    flags: AtomicFlag,
}

unsafe impl<T> Sync for TripleBuffer<T> {}

impl<T: Default> Default for TripleBuffer<T> {
    fn default() -> Self {
        Self {
            buffers: UnsafeCell::new([
                AlignedData(Default::default()),
                AlignedData(Default::default()),
                AlignedData(Default::default()),
            ]),
            flags: AtomicFlag::new(0x6),
        }
    }
}

impl<T> TripleBuffer<T> {
    /// Gets the index of the buffer to produce
    fn get_produce_index(&self) -> usize {
        (self.flags.load(Ordering::SeqCst) & 0x30) >> 4
    }

    /// Swaps consume and intermediate buffers and resets the new flag.
    /// If the new flag was set, the index to the (new) consume buffer is returned, otherwise None
    /// is returned.
    /// Index of the produce buffer is not modified.
    fn try_get_consume_index(&self) -> Option<usize> {
        let mut old_flags = self.flags.load(Ordering::Acquire);
        let mut new_flags: usize;
        loop {
            if (old_flags & 0x40) == 0 {
                // nothing new, no need to swap
                return None;
            }
            // clear the "new" bit and swap the indices of consume and intermediate buffers
            new_flags = (old_flags & 0x30) | ((old_flags & 0x3) << 2) | ((old_flags & 0xC) >> 2);

            match self
                .flags
                .compare_exchange(old_flags, new_flags, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(x) => old_flags = x,
            }
        }
        Some(new_flags & 0x3)
    }

    /// Swaps intermediate and (new)produced buffers and sets the new flag.
    /// Index of the consume buffer is not modified.
    fn set_produce(&self) {
        let mut old_flags = self.flags.load(Ordering::Acquire);
        loop {
            // set the "new" bit and swap the indices of produce and intermediate buffers
            let new_flags = 0x40 | ((old_flags & 0xC) << 2) | ((old_flags & 0x30) >> 2) | (old_flags & 0x3);

            match self
                .flags
                .compare_exchange(old_flags, new_flags, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(x) => old_flags = x,
            }
        }
    }
}

/// Sender part of the communication.
pub struct Sender<T>(Arc<TripleBuffer<T>>);

// The receiver can be sent from place to place, so long as it
// is not used to receive non-sendable things.
unsafe impl<T: Send> Send for Sender<T> {}

//impl<T> !Sync for Sender<T> { }

impl<T> Sender<T> {
    fn new(owner: &Arc<TripleBuffer<T>>) -> Sender<T> {
        Sender(owner.clone())
    }

    pub fn send_buffer(&self) -> RefSendBuffer<'_, T> {
        RefSendBuffer(&self.0, self.0.get_produce_index())
    }
}

impl<T: Copy> Sender<T> {
    pub fn send(&self, value: T) {
        let mut b = self.send_buffer();
        *b = value;
    }
}

/// Reference to the buffer held by the producer
pub struct RefSendBuffer<'a, T>(&'a TripleBuffer<T>, usize);

impl<'a, T> Drop for RefSendBuffer<'a, T> {
    fn drop(&mut self) {
        self.0.set_produce();
    }
}

impl<'a, T> Deref for RefSendBuffer<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &(*self.0.buffers.get())[self.1].0 }
    }
}

impl<'a, T> DerefMut for RefSendBuffer<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.0.buffers.get())[self.1].0 }
    }
}

/// Receiver part of the communication
pub struct Receiver<T>(Arc<TripleBuffer<T>>);

// The consumer can be sent from place to place, so long as it
// is not used to receive non-sendable things.
unsafe impl<T: Send> Send for Receiver<T> {}

//impl<T> !Sync for Receiver<T> { }

impl<T> Receiver<T> {
    fn new(owner: &Arc<TripleBuffer<T>>) -> Receiver<T> {
        Receiver(owner.clone())
    }

    pub fn receive_buffer(&self) -> Option<RefReceiveBuffer<'_, T>> {
        match self.0.try_get_consume_index() {
            Some(idx) => Some(RefReceiveBuffer(&self.0, idx)),
            None => None,
        }
    }
}

impl<T: Copy> Receiver<T> {
    pub fn receive(&self) -> Option<T> {
        self.receive_buffer().map(|b| *b)
    }
}

/// Reference to the buffer held by the consumer
pub struct RefReceiveBuffer<'a, T>(&'a TripleBuffer<T>, usize);

impl<'a, T> Deref for RefReceiveBuffer<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &(*self.0.buffers.get())[self.1].0 }
    }
}

impl<'a, T> DerefMut for RefReceiveBuffer<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.0.buffers.get())[self.1].0 }
    }
}

/// Create a Sender/Receiver with an embedded shared buffer for communication.
/// It is not a "Single Producer Single Consumer" queue as some massages might be dropped depending
/// on the thread scheduling.
pub fn state_channel<T: Default>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(TripleBuffer::default());
    (Sender::new(&a), Receiver::new(&a))
}
