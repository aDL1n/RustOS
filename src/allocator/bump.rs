use crate::allocator::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;
use x86_64::align_up;

pub struct BumpAllocator {
    heap_start: u64,
    heap_end: u64,
    next: u64,
    allocations: u64,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    pub unsafe fn init(&mut self, heap_start: u64, heap_size: u64) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }

    pub unsafe fn bump_alloc(&mut self, layout: Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align() as u64);
        let alloc_end = match alloc_start.checked_add(layout.size() as u64) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > self.heap_end {
            ptr::null_mut()
        } else {
            self.next = alloc_end;
            self.allocations += 1;

            alloc_start as *mut u8
        }
    }

    pub unsafe fn bump_dealloc(&mut self) {
        self.allocations -= 1;
        if self.allocations == 0 {
            self.next = self.heap_start;
        }
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { 
            self.lock().bump_alloc(layout) 
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        unsafe { self.lock().bump_dealloc() };
    }
}
