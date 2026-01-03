use alloc::alloc::{ GlobalAlloc, Layout };
use core::ptr;
use x86_64::align_up;
use crate::allocator::Locked;

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
            allocations: 0
        }
    }

    pub unsafe fn init(&mut self, heap_start: u64, heap_size: u64) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();
        
        let alloc_start = align_up(bump.next, layout.align() as u64);
        let alloc_end = match alloc_start.checked_add(layout.size() as u64) { 
            Some(end) => end,
            None => return ptr::null_mut(),
        };
        
        if alloc_end > bump.heap_end  {
            ptr::null_mut()
        } else { 
            bump.next = alloc_end;
            bump.allocations += 1;
            
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();
        
        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}