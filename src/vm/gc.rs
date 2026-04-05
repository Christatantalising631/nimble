use std::alloc::{alloc, dealloc, Layout};

pub struct BumpAllocator {
    pub start: *mut u8,
    pub cursor: *mut u8,
    pub end: *mut u8,
}

impl BumpAllocator {
    pub fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, 8).unwrap();
        let start = unsafe { alloc(layout) };
        Self {
            start,
            cursor: start,
            end: unsafe { start.add(size) },
        }
    }

    #[inline(always)]
    pub fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        let ptr = self.cursor;
        let new_cursor = unsafe { ptr.add(size) };
        if new_cursor > self.end {
            return None;
        }
        self.cursor = new_cursor;
        Some(ptr)
    }

    pub fn reset(&mut self) {
        self.cursor = self.start;
    }
}

pub struct Heap {
    pub nursery: BumpAllocator,
    pub threshold: usize,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            nursery: BumpAllocator::new(2 * 1024 * 1024), // 2MB
            threshold: 1 * 1024 * 1024,
        }
    }

    pub fn collect_minor(&mut self) {
        // Simple semi-space flip or reset for now
        self.nursery.reset();
    }
}

impl Drop for BumpAllocator {
    fn drop(&mut self) {
        let size = (self.end as usize) - (self.start as usize);
        let layout = Layout::from_size_align(size, 8).unwrap();
        unsafe { dealloc(self.start, layout) };
    }
}
