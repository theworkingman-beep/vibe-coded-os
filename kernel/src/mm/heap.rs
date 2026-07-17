//! Simple free-list heap allocator.
//!
//! Replaces the leak-only bump allocator with a basic allocator that can
//! actually free memory.  Allocations are rounded up to a power-of-two size
//! class and served from a per-class free list.  If no free block is
//! available, a new page (or multi-page block for large allocations) is fetched
//! from the physical frame allocator and split as needed.

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

const MIN_BLOCK_SIZE: usize = 16;
const MAX_BLOCK_SIZE: usize = 4096;
const NUM_CLASSES: usize = 9; // 16, 32, 64, 128, 256, 512, 1024, 2048, 4096

/// A free block header.  Stored in the first bytes of a free allocation.
#[repr(C)]
struct FreeBlock {
    next: *mut FreeBlock,
}

struct FreeList {
    head: *mut FreeBlock,
}

impl FreeList {
    const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
        }
    }

    unsafe fn push(&mut self, block: *mut FreeBlock) {
        (*block).next = self.head;
        self.head = block;
    }

    unsafe fn pop(&mut self) -> Option<*mut FreeBlock> {
        if self.head.is_null() {
            None
        } else {
            let node = self.head;
            self.head = (*node).next;
            Some(node)
        }
    }
}

struct Heap {
    classes: [FreeList; NUM_CLASSES],
    total_allocated: AtomicUsize,
    total_freed: AtomicUsize,
}

impl Heap {
    const fn new() -> Self {
        const EMPTY: FreeList = FreeList::new();
        Self {
            classes: [EMPTY; NUM_CLASSES],
            total_allocated: AtomicUsize::new(0),
            total_freed: AtomicUsize::new(0),
        }
    }

    fn class_index(size: usize) -> usize {
        let mut class_size = MIN_BLOCK_SIZE;
        for i in 0..NUM_CLASSES {
            if size <= class_size {
                return i;
            }
            class_size <<= 1;
        }
        NUM_CLASSES - 1
    }

    fn class_size(index: usize) -> usize {
        MIN_BLOCK_SIZE << index
    }

    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align()).max(MIN_BLOCK_SIZE);
        if size <= MAX_BLOCK_SIZE {
            let index = Self::class_index(size);
            let block_size = Self::class_size(index);
            if let Some(block) = self.classes[index].pop() {
                self.total_allocated
                    .fetch_add(block_size, Ordering::Relaxed);
                return block as *mut u8;
            }

            // No free block: fetch a fresh page and split it into blocks of this
            // class size, keeping one and pushing the rest onto the free list.
            let frame = match crate::mm::frame_allocator::allocate() {
                Some(f) => f,
                None => return ptr::null_mut(),
            };
            let page = crate::mm::hhdm::phys_to_virt(frame) as *mut u8;
            let page_size = 4096usize;
            let count = page_size / block_size;
            for i in 1..count {
                let block = page.add(i * block_size) as *mut FreeBlock;
                self.classes[index].push(block);
            }
            self.total_allocated
                .fetch_add(block_size, Ordering::Relaxed);
            page
        } else {
            // Large allocation: allocate whole pages directly.
            let pages = (size + 4095) / 4096;
            self.alloc_large(pages)
        }
    }

    unsafe fn alloc_large(&mut self, pages: usize) -> *mut u8 {
        // Use a fixed stack buffer for the frame list instead of a `Vec`.
        // Allocating a `Vec` here would re-enter the global allocator
        // (`HEAP.lock()` is already held by `alloc`), deadlocking on the
        // non-reentrant `spin::Mutex`. Bounded to 256 pages (1 MiB); larger
        // requests fail until a scatter-gather mapper lands.
        const MAX_LARGE_PAGES: usize = 256;
        if pages == 0 || pages > MAX_LARGE_PAGES {
            return ptr::null_mut();
        }
        let mut frames: [u64; MAX_LARGE_PAGES] = [0; MAX_LARGE_PAGES];
        let mut count = 0usize;
        for i in 0..pages {
            match crate::mm::frame_allocator::allocate() {
                Some(f) => {
                    frames[i] = f;
                    count += 1;
                }
                None => {
                    for j in 0..count {
                        crate::mm::frame_allocator::free(frames[j]);
                    }
                    return ptr::null_mut();
                }
            }
        }
        // Contiguous physical frames are not guaranteed, so this simple large
        // allocator relies on the frame allocator returning contiguous frames.
        // A real implementation would map scattered frames into a contiguous
        // virtual region using the page tables.
        let base = crate::mm::hhdm::phys_to_virt(frames[0]);
        self.total_allocated
            .fetch_add(pages * 4096, Ordering::Relaxed);
        base as *mut u8
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let size = layout.size().max(layout.align()).max(MIN_BLOCK_SIZE);
        if size <= MAX_BLOCK_SIZE {
            let index = Self::class_index(size);
            let block_size = Self::class_size(index);
            self.classes[index].push(ptr as *mut FreeBlock);
            self.total_freed.fetch_add(block_size, Ordering::Relaxed);
        } else {
            // Large deallocation: would need a side table to recover page count.
            // For this incremental Phase 2 implementation we leak large pages to
            // avoid unsafe assumptions about contiguity.
            crate::logln!(
                "heap: large deallocation at {:?} leaked (no side table)",
                ptr
            );
        }
    }
}

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

static HEAP: Mutex<Heap> = Mutex::new(Heap::new());

/// Global allocator backed by the free-list heap.
pub struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.lock().dealloc(ptr, layout);
    }
}

/// Total bytes currently allocated from the heap.
pub fn allocated_bytes() -> usize {
    HEAP.lock().total_allocated.load(Ordering::Relaxed)
        - HEAP.lock().total_freed.load(Ordering::Relaxed)
}
