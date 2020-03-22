use std::fmt;
use std::mem;
use std::ops;
use std::ptr;

enum Entry<T> {
    // Vacant entry pointing to the next free item (or usize::max_value if no empty item)
    Vacant(usize),
    Occupied(T),
}

/// Arena allocator
pub struct IndexedArena<T> {
    size: usize,
    items: Vec<Entry<T>>,
    free_head: Entry<T>,
    increment: usize,
}

impl<T> IndexedArena<T> {
    pub fn new() -> Self {
        IndexedArena {
            size: 0,
            items: Vec::new(),
            free_head: Entry::Vacant(usize::max_value()),
            increment: 0,
        }
    }

    pub fn new_with_capacity(capacity: usize, increment: usize) -> Self {
        let mut arena = Self::new();
        arena.increment = increment;
        arena.reserve(capacity);
        arena
    }

    pub fn reserve(&mut self, capacity: usize) {
        log::debug!("Increment capacity by {}", capacity);

        let start_length = self.items.len();
        if capacity > 0 {
            self.items.reserve_exact(capacity);
        }
        let capacity = self.items.capacity();
        unsafe { self.items.set_len(capacity) };
        for id in (start_length..self.items.len()).rev() {
            assert!(if let Entry::Vacant(_) = self.free_head {
                true
            } else {
                false
            });
            let head = mem::replace(&mut self.free_head, Entry::Vacant(id));
            unsafe { ptr::write(&mut self.items[id], head) };
        }
    }

    fn get_increment(&self) -> usize {
        if self.increment != 0 {
            return self.increment;
        }

        let len = self.items.len();
        if len <= 16 {
            16
        } else if len > 1204 {
            1024
        } else {
            len
        }
    }

    fn ensure_free(&mut self) {
        assert!(if let Entry::Vacant(_) = self.free_head {
            true
        } else {
            false
        });
        match self.free_head {
            Entry::Vacant(id) => {
                if id == usize::max_value() {
                    let increment = self.get_increment();
                    self.reserve(increment);
                }
            }
            _ => unreachable!(),
        }
        assert!(if let Entry::Vacant(_) = self.free_head {
            true
        } else {
            false
        });
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn allocate(&mut self, data: T) -> (usize, &mut T) {
        self.ensure_free();
        self.size += 1;
        let id = if let Entry::Vacant(id) = self.free_head {
            id
        } else {
            unreachable!()
        };
        self.free_head = mem::replace(&mut self.items[id], Entry::Occupied(data));
        assert!(if let Entry::Vacant(_) = self.free_head {
            true
        } else {
            false
        });
        if let Entry::Occupied(ref mut data) = &mut self.items[id] {
            (id, data)
        } else {
            unreachable!()
        }
    }

    pub fn deallocate(&mut self, id: usize) -> T {
        self.size -= 1;
        let head = mem::replace(&mut self.free_head, Entry::Vacant(id));
        let data = mem::replace(&mut self.items[id], head);
        if let Entry::Occupied(data) = data {
            data
        } else {
            panic!("Invalid index")
        }
    }

    pub fn clear(&mut self) {
        self.size = 0;
        self.items.clear();
        self.free_head = Entry::Vacant(usize::max_value());
        self.reserve(0); // relink all the allocated items in the veactor as free
    }
}

impl<T> Default for IndexedArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ops::Index<usize> for IndexedArena<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        if let Entry::Occupied(ref data) = &self.items[idx] {
            data
        } else {
            panic!()
        }
    }
}

impl<T> ops::IndexMut<usize> for IndexedArena<T> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        if let Entry::Occupied(ref mut data) = &mut self.items[idx] {
            data
        } else {
            panic!()
        }
    }
}

impl<T> fmt::Debug for IndexedArena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let free = if let Entry::Vacant(id) = self.free_head {
            id
        } else {
            unreachable!()
        };
        writeln!(f, "free: {}", free)?;
        writeln!(
            f,
            "size/len/cap: {}/{}/{}",
            self.size,
            self.items.len(),
            self.items.capacity()
        )?;

        write!(f, "[ ")?;
        for v in &self.items {
            match v {
                Entry::Vacant(id) => {
                    write!(f, "{} ", id)?;
                }
                _ => {
                    write!(f, "DATA ")?;
                }
            }
        }
        writeln!(f, "]")?;
        writeln!(f)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::IndexedArena;
    use permutohedron::Heap;
    use rand;
    use rand::seq::SliceRandom;
    use std::cell::Cell;
    use std::mem;

    struct DropTracker<'a>(&'a Cell<usize>);

    impl<'a> Drop for DropTracker<'a> {
        fn drop(&mut self) {
            log::trace!("drop");
            self.0.set(self.0.get() + 1);
        }
    }

    struct Node<'a>(i32, DropTracker<'a>);

    #[test]
    fn simple() {
        let drop_counter = Cell::new(0);
        {
            let mut arena = IndexedArena::new();

            log::debug!("store");
            assert_eq!(arena.len(), 0);

            let (id1, _) = arena.allocate(Node(1, DropTracker(&drop_counter)));
            assert_eq!(arena.len(), 1);

            let (id2, _) = arena.allocate(Node(2, DropTracker(&drop_counter)));
            assert_eq!(arena.len(), 2);

            let (id3, _) = arena.allocate(Node(3, DropTracker(&drop_counter)));
            assert_eq!(arena.len(), 3);

            let (id4, _) = arena.allocate(Node(4, DropTracker(&drop_counter)));
            assert_eq!(arena.len(), 4);

            assert_eq!(arena[id1].0, 1);
            assert_eq!(arena[id2].0, 2);
            assert_eq!(arena[id3].0, 3);
            assert_eq!(arena[id4].0, 4);
            assert_eq!(drop_counter.get(), 0);

            log::debug!("remove");
            let node3 = arena.deallocate(id3);
            assert_eq!(arena.len(), 3);
            assert_eq!(drop_counter.get(), 0);
            mem::drop(node3);
            assert_eq!(drop_counter.get(), 1);

            log::debug!("add");
            let (id3, _) = arena.allocate(Node(103, DropTracker(&drop_counter)));
            assert_eq!(arena.len(), 4);

            assert_eq!(arena[id1].0, 1);
            assert_eq!(arena[id2].0, 2);
            assert_eq!(arena[id3].0, 103);
            assert_eq!(arena[id4].0, 4);
        }
        assert_eq!(drop_counter.get(), 5);
    }

    #[test]
    fn stress() {
        let mut data = [1usize, 2, 5, 7, 100, 4000];

        let mut heap = Heap::new(&mut data);
        while let Some(sizes) = heap.next_permutation() {
            log::trace!("permutation {:?}", sizes);

            let drop_counter = Cell::new(0);
            let mut drop_count = 0;
            {
                let mut arena = IndexedArena::new();

                for &mut cnt in sizes.into_iter() {
                    let rem = cnt / 2;
                    let mut ids = Vec::new();

                    log::trace!("store {}", cnt);
                    for i in 0..cnt {
                        assert_eq!(arena.len(), i);
                        let (id, _) = arena.allocate(Node(i as i32, DropTracker(&drop_counter)));
                        ids.push((i as i32, id));
                    }
                    assert_eq!(arena.len(), cnt);
                    assert_eq!(drop_counter.get(), drop_count);

                    ids.shuffle(&mut rand::thread_rng());

                    log::trace!("check");
                    for v in ids.iter() {
                        assert_eq!(arena[v.1].0, v.0);
                    }

                    log::trace!("remove half");
                    for i in 0..rem {
                        assert_eq!(drop_counter.get(), drop_count + i);
                        assert_eq!(arena.len(), cnt - i);
                        let d = arena.deallocate(ids[i].1);
                        mem::drop(d);
                        ids[i].1 = usize::max_value();
                    }
                    assert_eq!(arena.len(), cnt - rem);
                    assert_eq!(drop_counter.get(), drop_count + rem);

                    log::trace!("check");
                    for v in ids.iter() {
                        if v.1 != usize::max_value() {
                            assert_eq!(arena[v.1].0, v.0);
                        }
                    }

                    log::trace!("add back");
                    for v in ids.iter_mut() {
                        if v.1 == usize::max_value() {
                            let (id, _) = arena.allocate(Node(-v.0, DropTracker(&drop_counter)));
                            v.1 = id;
                        }
                    }
                    assert_eq!(arena.len(), ids.len());
                    assert_eq!(drop_counter.get(), drop_count + rem);

                    log::trace!("check");
                    for v in ids.iter() {
                        assert!(arena[v.1].0 == v.0 || arena[v.1].0 == -v.0);
                    }

                    arena.clear();
                    assert_eq!(arena.len(), 0);
                    assert_eq!(drop_counter.get(), drop_count + rem + cnt);
                    drop_count += rem + cnt;
                }
            }
            assert_eq!(drop_counter.get(), drop_count);
        }
    }
}
