use core::ops::{Drop, FnOnce};

pub struct Finally<F>
where
    F: FnOnce(),
{
    finally: Option<F>,
}

impl<F> Drop for Finally<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        if let Some(finally) = self.finally.take() {
            finally()
        }
    }
}

pub fn finally<F>(finally: F) -> Finally<F>
where
    F: FnOnce(),
{
    Finally { finally: Some(finally) }
}
