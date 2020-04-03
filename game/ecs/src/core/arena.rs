use std::fmt;
use std::mem;
use std::ops;
use std::ptr;

enum Entry<T> {
    // Vacant entry pointing to the next free item (or usize::max_value if no empty item)
    Vacant(usize),
    Occupied(T),
}

impl<T> Entry<T> {
    fn is_vacant(&self) -> bool {
        match self {
            &Entry::Vacant(_) => true,
            _ => false,
        }
    }
}

/// Arena allocator
pub struct Arena<T> {
    size: usize,
    pages: Vec<Vec<Entry<T>>>,
    free_head: Entry<T>,
    page_size: usize,
}

impl<T> Arena<T> {
    pub fn new(page_size: usize) -> Self {
        Arena {
            size: 0,
            pages: Vec::new(),
            free_head: Entry::Vacant(usize::max_value()),
            page_size,
        }
    }

    fn add_page(&mut self) {
        let start_id = self.pages.len() * self.page_size;
        let mut page = Vec::with_capacity(self.page_size);
        unsafe { page.set_len(self.page_size) };
        for i in (0..self.page_size).rev() {
            let id = start_id + i;
            assert!(self.free_head.is_vacant());
            let head = mem::replace(&mut self.free_head, Entry::Vacant(id));
            unsafe { ptr::write(&mut page[i], head) };
        }
        self.pages.push(page);
    }

    fn ensure_free(&mut self) {
        debug_assert!(self.free_head.is_vacant());
        match self.free_head {
            Entry::Vacant(id) => {
                if id == usize::max_value() {
                    self.add_page();
                }
            }
            _ => unreachable!(),
        }
        debug_assert!(self.free_head.is_vacant());
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn allocate(&mut self, data: T) -> (usize, &mut T) {
        self.ensure_free();
        let (id, p, pi) = if let Entry::Vacant(id) = self.free_head {
            (id, id / self.page_size, id % self.page_size)
        } else {
            unreachable!()
        };
        self.free_head = mem::replace(&mut self.pages[p][pi], Entry::Occupied(data));
        debug_assert!(self.free_head.is_vacant());
        if let Entry::Occupied(ref mut data) = &mut self.pages[p][pi] {
            self.size += 1;
            (id, data)
        } else {
            unreachable!()
        }
    }

    pub fn deallocate(&mut self, id: usize) -> T {
        let (p, pi) = (id / self.page_size, id % self.page_size);
        let head = mem::replace(&mut self.free_head, Entry::Vacant(id));
        let data = mem::replace(&mut self.pages[p][pi], head);
        if let Entry::Occupied(data) = data {
            self.size -= 1;
            data
        } else {
            panic!("Invalid index")
        }
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.size = 0;
        self.free_head = Entry::Vacant(usize::max_value());
    }
}

impl<T> ops::Index<usize> for Arena<T> {
    type Output = T;

    fn index(&self, id: usize) -> &Self::Output {
        let (p, pi) = (id / self.page_size, id % self.page_size);
        if let Entry::Occupied(ref data) = &self.pages[p][pi] {
            data
        } else {
            panic!()
        }
    }
}

impl<T> ops::IndexMut<usize> for Arena<T> {
    fn index_mut(&mut self, id: usize) -> &mut Self::Output {
        let (p, pi) = (id / self.page_size, id % self.page_size);
        if let Entry::Occupied(ref mut data) = &mut self.pages[p][pi] {
            data
        } else {
            panic!()
        }
    }
}

impl<T> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let free = if let Entry::Vacant(id) = self.free_head {
            id
        } else {
            unreachable!()
        };
        writeln!(f, "free: {}", free)?;
        writeln!(
            f,
            "size/pages/all: {}/{}/{}",
            self.size,
            self.pages.len(),
            self.pages.len() * self.page_size
        )?;

        write!(f, "[ ")?;
        for page in &self.pages {
            for v in page {
                match v {
                    Entry::Vacant(id) => {
                        write!(f, "{} ", id)?;
                    }
                    _ => {
                        write!(f, "DATA ")?;
                    }
                }
            }
        }
        writeln!(f, "]")?;
        writeln!(f)?;
        Ok(())
    }
}
