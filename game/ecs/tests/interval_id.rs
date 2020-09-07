use permutohedron::Heap;
use shine_ecs::scheduler::IntervalStore;

mod utils;

#[test]
fn alloc_dealloc_stress() {
    utils::init_logger();

    const ALLOC_COUNT: usize = 7;
    const DEALLOC_COUNT: usize = 4;

    let mut data = (0..ALLOC_COUNT).collect::<Vec<_>>();
    let mut heap = Heap::new(&mut data);
    while let Some(order) = heap.next_permutation() {
        let mut interval = IntervalStore::new(0..ALLOC_COUNT as u32);
        let mut allocations = Vec::with_capacity(ALLOC_COUNT);
        let mut size = 0;

        log::trace!("order: {:?}", order);

        log::debug!("Allocating...");
        log::trace!("before allocating: {:?}", interval);
        for _ in 0..ALLOC_COUNT {
            let id = interval.allocate();
            log::trace!("after allocation of {:?}:  {:?}", id, interval);
            allocations.push(id);

            size += 1;
            assert!(interval.allocation_count() == size);
        }
        assert!(interval.is_full());

        log::debug!("Deallocating some...");
        log::trace!("before deallocating some: {:?}", interval);
        for i in 0..DEALLOC_COUNT {
            let id = allocations[order[i]].clone();
            log::trace!("deallocate {:?} from {:?}", id, interval);
            interval.deallocate(id);

            size -= 1;
            assert!(interval.allocation_count() == size);
        }

        log::debug!("Allocating some...");
        log::trace!("before allocating some: {:?}", interval);
        for i in 0..DEALLOC_COUNT {
            let id = interval.allocate();
            log::trace!("after allocation of {:?}:  {:?}", id, interval);
            allocations[order[i]] = id;

            size += 1;
            assert!(interval.allocation_count() == size);
        }
        assert!(interval.is_full());

        log::debug!("Deallocating...");
        log::trace!("before deallocating: {:?}", interval);
        for i in 0..ALLOC_COUNT {
            let id = allocations[order[i]].clone();
            log::trace!("deallocate {:?} from {:?}", id, interval);
            interval.deallocate(id);

            size -= 1;
            assert!(interval.allocation_count() == size);
        }
        assert!(interval.is_empty());
    }
}
