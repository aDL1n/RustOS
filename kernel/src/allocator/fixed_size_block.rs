use crate::allocator::bump::BumpAllocator;
use crate::allocator::Locked;
use core::alloc::{GlobalAlloc, Layout};
use core::mem;

struct ListNode {
    next: Option<&'static mut ListNode>,
}

const BLOCK_SIZES: &[u64] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];
const MAX_FREE_BLOCKS_PER_SIZE: u64 = 128;

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    free_blocks_counts: [u64; BLOCK_SIZES.len()],
    fallback_allocator: BumpAllocator,
}

impl FixedSizeBlockAllocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            free_blocks_counts: [0; BLOCK_SIZES.len()],
            fallback_allocator: BumpAllocator::new(),
        }
    }

    pub unsafe fn init(&mut self, heap_start: u64, heap_size: u64) {
        unsafe {
            self.fallback_allocator.init(heap_start, heap_size);
        }
    }

    fn alloc_from_list(&mut self, block_size_index: usize) -> *mut u8 {
        match self.list_heads[block_size_index].take() {
            Some(node) => {
                self.free_blocks_counts[block_size_index] -= 1;
                self.list_heads[block_size_index] = node.next.take();

                node as *mut ListNode as *mut u8
            }
            None => {
                let block_size = BLOCK_SIZES[block_size_index] as usize;
                let layout = Layout::from_size_align(block_size, block_size).unwrap();

                self.fallback_alloc(layout)
            }
        }
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        unsafe { self.fallback_allocator.bump_alloc(layout) }
    }
}

fn list_index(layout: &Layout) -> Option<usize> {
    let required_size = layout.size().max(layout.align());

    BLOCK_SIZES.iter().position(|&s| s >= required_size as u64)
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        match list_index(&layout) {
            Some(index) => allocator.alloc_from_list(index),
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        match list_index(&layout) {
            Some(index) => {
                if allocator.free_blocks_counts[index] >= MAX_FREE_BLOCKS_PER_SIZE {
                    return;
                }
                
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };

                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index] as usize);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index] as usize);

                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                    allocator.free_blocks_counts[index] += 1;
                }
            }
            None => {},
        }
    }
}
