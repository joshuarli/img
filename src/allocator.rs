//! A global allocator that returns large blocks to the operating system on free.

use std::alloc::{GlobalAlloc, Layout, System};

/// Scoring and encoding churn through tens of megabytes of short-lived planes
/// per candidate. Apple's libmalloc keeps freed large blocks dirty in a reuse
/// cache (about 29 B/px for a scored 8 MP JPEG) and glibc's adaptive mmap
/// threshold moves them into arenas it rarely trims; both count toward the
/// footprint that mobile and container memory limits enforce. Blocks of at
/// least [`MAPPED_THRESHOLD`] bytes are therefore mapped and unmapped directly;
/// smaller blocks use the system allocator.
pub struct PageReturningAllocator;

/// Smallest allocation served by direct anonymous mappings.
pub const MAPPED_THRESHOLD: usize = 1 << 20;
const PAGE: usize = 4096;

fn mapped(layout: Layout) -> bool { layout.size() >= MAPPED_THRESHOLD && layout.align() <= PAGE }

unsafe fn map(size: usize) -> *mut u8 {
    // SAFETY: an anonymous private mapping has no preconditions; failure is reported as MAP_FAILED.
    let p = unsafe { libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0) };
    if p == libc::MAP_FAILED { std::ptr::null_mut() } else { p.cast() }
}

// SAFETY: every block is released by the same strategy that allocated it, chosen from its layout.
unsafe impl GlobalAlloc for PageReturningAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if mapped(layout) { unsafe { map(layout.size()) } } else { unsafe { System.alloc(layout) } }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if mapped(layout) { unsafe { map(layout.size()) } } else { unsafe { System.alloc_zeroed(layout) } }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if mapped(layout) { unsafe { libc::munmap(ptr.cast(), layout.size()) }; } else { unsafe { System.dealloc(ptr, layout) } }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        if !mapped(layout) && !mapped(new_layout) { return unsafe { System.realloc(ptr, layout, new_size) }; }
        let new = unsafe { self.alloc(new_layout) };
        if !new.is_null() {
            unsafe { std::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size)) };
            unsafe { self.dealloc(ptr, layout) };
        }
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_keep_contents_across_the_mapped_threshold() {
        let a = PageReturningAllocator;
        for (from, to) in [(64, MAPPED_THRESHOLD + 7), (MAPPED_THRESHOLD + 7, 3 * MAPPED_THRESHOLD), (3 * MAPPED_THRESHOLD, 100), (MAPPED_THRESHOLD - 1, MAPPED_THRESHOLD)] {
            unsafe {
                let layout = Layout::from_size_align(from, 16).unwrap();
                let p = a.alloc(layout);
                assert!(!p.is_null());
                for i in 0..from { *p.add(i) = (i % 251) as u8; }
                let q = a.realloc(p, layout, to);
                assert!(!q.is_null());
                for i in 0..from.min(to) { assert_eq!(*q.add(i), (i % 251) as u8); }
                a.dealloc(q, Layout::from_size_align(to, 16).unwrap());
            }
        }
        unsafe {
            let layout = Layout::from_size_align(2 * MAPPED_THRESHOLD, 4096).unwrap();
            let z = a.alloc_zeroed(layout);
            assert!((0..layout.size()).step_by(4093).all(|i| *z.add(i) == 0) && z as usize % 4096 == 0);
            a.dealloc(z, layout);
        }
    }
}
