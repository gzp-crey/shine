use crate::dbg_assert;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntervalId(u32);

impl IntervalId {
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Keep track of used/unused ids within the given interval.
/// A sorted list of empty region is used internally.
#[derive(Debug)]
pub struct IntervalStore {
    range: Range<u32>,
    size: usize,

    /// sorted set of empty intervals
    empty: Vec<(u32, u32)>,
}

impl IntervalStore {
    pub fn new(range: Range<u32>) -> IntervalStore {
        let empty = vec![(range.start, range.end)];
        IntervalStore { range, size: 0, empty }
    }

    pub fn allocation_count(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn is_full(&self) -> bool {
        self.empty.is_empty()
    }

    pub fn allocate(&mut self) -> IntervalId {
        dbg_assert!(self.check_intervals());
        let entry = &mut self.empty.first_mut().expect("Access Intervall is full");
        assert!(entry.1 > entry.0);
        let id = entry.0;
        entry.0 += 1;
        if entry.0 >= entry.1 {
            self.empty.retain(|e| e.0 < e.1);
        }
        self.size += 1;
        assert!(self.range.contains(&id));
        dbg_assert!(self.check_intervals());
        IntervalId(id)
    }

    pub fn deallocate(&mut self, id: IntervalId) {
        assert!(self.range.contains(&id.0));
        dbg_assert!(self.check_intervals());

        // insert new empty interval
        match self.empty.iter().position(|e| id.0 < e.1) {
            None => {
                self.empty.push((id.0, id.0 + 1));
            }
            Some(i) => {
                assert!(self.empty[i].0 > id.0); // already released
                self.empty.insert(i, (id.0, id.0 + 1));
            }
        };

        self.size -= 1;

        //merge empty intervals
        for i in (1..self.empty.len()).rev() {
            let e1 = self.empty[i];
            let merged = {
                let mut e0 = &mut self.empty[i - 1];
                if e0.1 == e1.0 {
                    e0.1 = e1.1;
                    true
                } else {
                    false
                }
            };

            if merged {
                self.empty.remove(i);
            }
        }
        dbg_assert!(self.check_intervals());
    }

    #[cfg(debug_assertions)]
    fn check_intervals(&self) -> bool {
        for i in 1..self.empty.len() {
            let e0 = self.empty[i - 1];
            let e1 = self.empty[i];
            if e0.0 >= e0.1 {
                return false;
            }
            if e1.0 >= e1.1 {
                return false;
            }
            if e0.1 >= e1.0 {
                return false;
            }
        }
        true
    }
}
