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

impl<T: Default> TripleBuffer<T> {
    pub fn new() -> TripleBuffer<T> {
        TripleBuffer {
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
    /// If a the new flag was set, the index to the (new) consume buffer is returned, otherwise Err
    /// is returned.
    /// Index of the produce buffer is not modified.
    fn try_get_consume_index(&self) -> Result<usize, ()> {
        let mut old_flags = self.flags.load(Ordering::Acquire);
        let mut new_flags: usize;
        loop {
            if (old_flags & 0x40) == 0 {
                // nothing new, no need to swap
                return Err(());
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
        Ok(new_flags & 0x3)
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

    pub fn send_buffer(&self) -> Result<RefSendBuffer<'_, T>, ()> {
        Ok(RefSendBuffer(&self.0, self.0.get_produce_index()))
    }
}

impl<T: Copy> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), ()> {
        match self.send_buffer() {
            Ok(mut b) => {
                *b = value;
                Ok(())
            }
            Err(_) => Err(()),
        }
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

    pub fn receive_buffer(&self) -> Result<RefReceiveBuffer<'_, T>, ()> {
        match self.0.try_get_consume_index() {
            Ok(idx) => Ok(RefReceiveBuffer(&self.0, idx)),
            Err(_) => Err(()),
        }
    }
}

impl<T: Copy> Receiver<T> {
    pub fn receive(&self) -> Result<T, ()> {
        match self.receive_buffer() {
            Ok(b) => Ok(*b),
            Err(_) => Err(()),
        }
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
    let a = Arc::new(TripleBuffer::new());
    (Sender::new(&a), Receiver::new(&a))
}

#[cfg(test)]
mod test {
    use super::state_channel;
    use std::{env, thread};

    const ITER_COUNT: i32 = 0x2ffff;

    #[test]
    fn single_threaded_logic() {
        let (p, c) = state_channel();

        assert!(c.receive().is_err());

        p.send(1).unwrap();
        assert_eq!(c.receive().unwrap(), 1);
        assert!(c.receive().is_err());
        assert!(c.receive().is_err());

        p.send(2).unwrap();
        assert_eq!(c.receive().unwrap(), 2);
        assert!(c.receive().is_err());
        assert!(c.receive().is_err());

        p.send(3).unwrap();
        assert_eq!(c.receive().unwrap(), 3);
        assert!(c.receive().is_err());
        assert!(c.receive().is_err());

        p.send(4).unwrap();
        assert_eq!(c.receive().unwrap(), 4);
        assert!(c.receive().is_err());
        assert!(c.receive().is_err());
    }

    #[test]
    fn single_threaded_stress_small_buffer() {
        let (p, c) = state_channel();

        for x in 0..ITER_COUNT {
            p.send(x).unwrap();
            assert_eq!(c.receive().unwrap(), x);
        }
    }

    #[test]
    fn multi_threaded_stress_small_buffer() {
        assert!(
            env::args().any(|a| a == "--test-threads=1")
                || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1"
        );

        let (p, c) = state_channel();

        let tp = thread::spawn(move || {
            for x in 0..ITER_COUNT {
                p.send(x).unwrap();
            }
            //println!("produced: {}", ITER_COUNT);
        });
        let tc = thread::spawn(move || {
            let mut prev = -1;
            let mut _cnt = 0;
            loop {
                match c.receive() {
                    Ok(x) => {
                        _cnt += 1;
                        assert!(prev < x);
                        prev = x;
                        if prev == ITER_COUNT - 1 {
                            break;
                        }
                    }
                    Err(_) => {}
                }
            }
            //println!("consumed: {}", _cnt);
        });

        tp.join().unwrap();
        tc.join().unwrap();
    }

    fn is_prime(n: i32) -> bool {
        if n == 2 || n == 3 {
            return true;
        } else if n % 2 == 0 || n % 3 == 0 {
            return false;
        }

        let mut i = 5;
        let mut res = true;
        while i * i <= n {
            if n % i == 0 {
                res = false;
            }
            i = i + 1;
        }
        res
    }

    fn long_calc(x: i32) -> i32 {
        if is_prime(x) {
            11
        } else {
            87
        }
    }

    struct BigData {
        pre: i32,
        x: i32,
        data: [i32; 64],
        post: i32,
    }

    impl Default for BigData {
        fn default() -> BigData {
            BigData {
                pre: 2,
                x: 0,
                data: [0; 64],
                post: 2,
            }
        }
    }

    #[test]
    fn single_threaded_stress_big_buffer() {
        let (p, c) = state_channel::<BigData>();
        for x in 0..ITER_COUNT {
            {
                let mut d = p.send_buffer().unwrap();
                assert_eq!(d.pre, 2);
                d.pre = 1;
                for i in 0..d.data.len() {
                    d.data[i] = x;
                }
                assert_eq!(d.post, 2);
                d.post = 1;
            }

            {
                let mut d = c.receive_buffer().unwrap();
                assert_eq!(d.pre, 1);
                assert_eq!(d.post, 1);
                d.pre = 2;
                for i in 0..d.data.len() {
                    assert_eq!(d.data[i], x);
                }
                d.post = 2;
            }
        }
    }

    #[test]
    fn multi_threaded_stress_big_buffer() {
        assert!(
            env::args().any(|a| a == "--test-threads=1")
                || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1"
        );

        let (p, c) = state_channel::<BigData>();

        let tp = thread::spawn(move || {
            for x in 0..ITER_COUNT {
                let mut d = p.send_buffer().unwrap();
                d.pre = 1;
                d.x = x;
                for i in 0..d.data.len() {
                    d.data[i] = long_calc(x);
                }
                d.post = 1;
                assert_eq!(d.pre, 1);
                assert_eq!(d.post, 1);
            }
            log::info!("produced: {}", ITER_COUNT);
        });
        let tc = thread::spawn(move || {
            let mut prev = -1;
            let mut cnt = 0;
            loop {
                match c.receive_buffer() {
                    Ok(mut d) => {
                        cnt += 1;
                        assert_eq!(d.pre, 1);
                        assert_eq!(d.post, 1);
                        d.pre = 2;
                        for i in 0..d.data.len() {
                            assert_eq!(d.data[i], d.data[0]);
                        }
                        d.post = 2;
                        assert_eq!(d.pre, 2);
                        assert_eq!(d.post, 2);
                        assert!(prev < d.x);
                        prev = d.x;
                        if prev == ITER_COUNT - 1 {
                            break;
                        }
                    }
                    Err(_) => {}
                }
            }
            log::info!("consumed: {}", cnt);
        });

        tp.join().unwrap();
        tc.join().unwrap();
    }
}
