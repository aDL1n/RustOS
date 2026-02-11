use super::{align_up, Locked};
use core::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};

struct ListNode {
    size: u64,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: u64) -> ListNode {
        ListNode { size, next: None }
    }
    
    fn start_address(&self) -> u64 {
        self as *const Self as u64
    }
    
    fn end_address(&self) -> u64 {
        self.start_address() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }
    
    pub unsafe fn init(&mut self, heap_start: u64, heap_size: u64) {
        self.add_free_region(heap_start, heap_size);
    }

    fn add_free_region(&mut self, addr: u64, size: u64) {
        assert_eq!(align_up(addr, mem::align_of::<ListNode>() as u64), addr);
        assert!(size >= mem::size_of::<ListNode>() as u64);

        let mut current = &mut self.head;
        while current.next.as_ref()
            .map_or(false, |n| n.start_address() < addr)
        {
            current = current.next.as_mut().unwrap();
        }

        if current.size > 0 {
            assert!(current.end_address() <= addr, "Memory corruption: overlapping free block!");
        }

        let mut next = current.next.take();
        let mut new_size = size;

        if let Some(ref mut node) = next {
            assert!(
                addr + size <= node.start_address(),
                "Memory corruption: overlapping free block!");

            if addr + size == node.start_address() {
                new_size += node.size;
                next = node.next.take();
            }
        }

        if current.size > 0 && current.end_address() == addr {
            current.size += new_size;
            current.next = next;
        } else {
            let node_ptr = addr as *mut ListNode;

            unsafe {
                node_ptr.write(ListNode {
                    size: new_size,
                    next
                });
                current.next = Some(&mut *node_ptr);
            }
        }
    }

    fn find_region(&mut self, size: u64, align: u64) -> Option<(&'static mut ListNode, u64)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take();

                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;

                return ret;
            } else {
                current = current.next.as_mut().unwrap();
            }
        }

        None
    }

    fn alloc_from_region(region: &ListNode, size: u64, align: u64) -> Result<u64, ()> {
        let alloc_start = align_up(region.start_address(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_address() {
            return Err(());
        }

        let excess_size = region.end_address() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() as u64 {
            return Err(());
        }

        Ok(alloc_start)
    }

    fn size_align(layout: Layout) -> (u64, u64) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());

        (size as u64, layout.align() as u64)
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).ok_or(()).expect("overflow");
            let excess_size = region.end_address() - alloc_end;

            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);

        self.lock().add_free_region(ptr as u64, size);
    }
}