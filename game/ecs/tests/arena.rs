use permutohedron::Heap;
use rand;
use rand::seq::SliceRandom;
use shine_ecs::core::arena::Arena;
use std::cell::Cell;
use std::mem;

mod utils;

struct DropTracker<'a>(&'a Cell<usize>);

impl<'a> Drop for DropTracker<'a> {
    fn drop(&mut self) {
        //log::trace!("drop");
        self.0.set(self.0.get() + 1);
    }
}

struct Node<'a>(i32, DropTracker<'a>);

#[test]
fn simple() {
    utils::init_logger();

    let drop_counter = Cell::new(0);
    {
        let mut arena = Arena::new(2);

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
    utils::init_logger();

    let mut data = [1usize, 2, 5, 7, 100, 4000];

    let mut heap = Heap::new(&mut data);
    while let Some(sizes) = heap.next_permutation() {
        log::info!("permutation {:?}", sizes);

        let drop_counter = Cell::new(0);
        let mut drop_count = 0;
        {
            let mut arena = Arena::new(16);

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

                log::trace!("clear");
                arena.clear();
                assert_eq!(arena.len(), 0);
                assert_eq!(drop_counter.get(), drop_count + rem + cnt);
                drop_count += rem + cnt;
            }
        }
        assert_eq!(drop_counter.get(), drop_count);
    }
}
