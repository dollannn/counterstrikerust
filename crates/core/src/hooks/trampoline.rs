//! Trampoline memory allocation
//!
//! Allocates executable memory within ±2GB of target addresses for relative jumps.

use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::ptr::NonNull;

/// Page size (4KB on most systems)
const PAGE_SIZE: usize = 4096;

/// Trampoline allocation size (enough for most hooks)
const TRAMPOLINE_SIZE: usize = 64;

/// Maximum search range for near allocation (2GB)
const MAX_RANGE: usize = 0x7FFF_0000;

/// Global trampoline allocator
static ALLOCATOR: Mutex<TrampolineAllocator> = Mutex::new(TrampolineAllocator::new());

/// Allocator for executable trampolines
struct TrampolineAllocator {
    /// Pages allocated, keyed by base address
    pages: BTreeMap<usize, PageInfo>,
}

struct PageInfo {
    base: *mut u8,
    size: usize,
    used: usize,
}

// SAFETY: The allocator is protected by a mutex and pages are only accessed through it
unsafe impl Send for PageInfo {}

impl TrampolineAllocator {
    const fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
        }
    }

    /// Allocate a trampoline near the target address
    fn alloc_near(&mut self, target: usize, size: usize) -> Option<NonNull<u8>> {
        // First, try to find an existing page within range
        for (&base, page) in &mut self.pages {
            let offset = if base > target {
                base - target
            } else {
                target - base
            };

            if offset < MAX_RANGE && page.used + size <= page.size {
                let ptr = unsafe { page.base.add(page.used) };
                page.used += size;
                return NonNull::new(ptr);
            }
        }

        // Allocate a new page near the target
        let new_page = self.alloc_page_near(target)?;
        let page = self.pages.get_mut(&(new_page as usize))?;

        let ptr = new_page;
        page.used = size;
        NonNull::new(ptr)
    }

    #[cfg(unix)]
    fn alloc_page_near(&mut self, target: usize) -> Option<*mut u8> {
        use nix::sys::mman::{mmap_anonymous, MapFlags, ProtFlags};
        use std::num::NonZeroUsize;

        let search_start = target.saturating_sub(MAX_RANGE);
        let search_end = target.saturating_add(MAX_RANGE);

        // Try allocating at hint addresses within range
        for hint in (search_start..search_end).step_by(PAGE_SIZE * 64) {
            // Skip invalid addresses
            if hint == 0 {
                continue;
            }

            let result = unsafe {
                mmap_anonymous(
                    NonZeroUsize::new(hint),
                    NonZeroUsize::new_unchecked(PAGE_SIZE),
                    ProtFlags::PROT_READ | ProtFlags::PROT_WRITE | ProtFlags::PROT_EXEC,
                    MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
                )
            };

            if let Ok(ptr) = result {
                let base = ptr.as_ptr() as *mut u8;
                let actual_addr = base as usize;

                // Verify the allocation is within range
                let offset = if actual_addr > target {
                    actual_addr - target
                } else {
                    target - actual_addr
                };

                if offset < MAX_RANGE {
                    self.pages.insert(
                        actual_addr,
                        PageInfo {
                            base,
                            size: PAGE_SIZE,
                            used: 0,
                        },
                    );
                    return Some(base);
                } else {
                    // Allocation was too far, unmap it
                    unsafe {
                        let _ = nix::sys::mman::munmap(ptr, PAGE_SIZE);
                    }
                }
            }
        }

        // Try without hint as a fallback
        let result = unsafe {
            mmap_anonymous(
                None,
                NonZeroUsize::new_unchecked(PAGE_SIZE),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE | ProtFlags::PROT_EXEC,
                MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
            )
        };

        if let Ok(ptr) = result {
            let base = ptr.as_ptr() as *mut u8;
            let actual_addr = base as usize;
            self.pages.insert(
                actual_addr,
                PageInfo {
                    base,
                    size: PAGE_SIZE,
                    used: 0,
                },
            );
            tracing::warn!(
                "Trampoline allocation fallback: allocated at {:x} for target {:x}",
                actual_addr,
                target
            );
            return Some(base);
        }

        tracing::error!("Failed to allocate page near {:x}", target);
        None
    }

    #[cfg(windows)]
    fn alloc_page_near(&mut self, target: usize) -> Option<*mut u8> {
        use windows::Win32::System::Memory::*;

        let search_start = target.saturating_sub(MAX_RANGE);
        let search_end = target.saturating_add(MAX_RANGE);

        for hint in (search_start..search_end).step_by(PAGE_SIZE * 64) {
            if hint == 0 {
                continue;
            }

            let result = unsafe {
                VirtualAlloc(
                    Some(hint as *const std::ffi::c_void),
                    PAGE_SIZE,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_EXECUTE_READWRITE,
                )
            };

            if !result.is_null() {
                let base = result as *mut u8;
                let actual_addr = base as usize;

                let offset = if actual_addr > target {
                    actual_addr - target
                } else {
                    target - actual_addr
                };

                if offset < MAX_RANGE {
                    self.pages.insert(
                        actual_addr,
                        PageInfo {
                            base,
                            size: PAGE_SIZE,
                            used: 0,
                        },
                    );
                    return Some(base);
                } else {
                    unsafe {
                        let _ = VirtualFree(result, 0, MEM_RELEASE);
                    }
                }
            }
        }

        tracing::error!("Failed to allocate page near {:x}", target);
        None
    }
}

/// Allocate a trampoline buffer near the target address
pub fn alloc_trampoline(target: *const u8) -> Option<NonNull<u8>> {
    ALLOCATOR
        .lock()
        .alloc_near(target as usize, TRAMPOLINE_SIZE)
}

/// Allocate a trampoline buffer of specific size near the target
pub fn alloc_trampoline_sized(target: *const u8, size: usize) -> Option<NonNull<u8>> {
    ALLOCATOR.lock().alloc_near(target as usize, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trampoline_allocation() {
        // Test basic allocation
        let target = 0x7FFF_0000_0000usize as *const u8;
        let trampoline = alloc_trampoline(target);
        assert!(trampoline.is_some(), "Should allocate trampoline");

        let ptr = trampoline.unwrap().as_ptr();
        assert!(!ptr.is_null(), "Trampoline should not be null");
    }

    #[test]
    fn test_multiple_allocations() {
        let target = 0x7FFF_0000_1000usize as *const u8;

        // Allocate multiple trampolines
        let t1 = alloc_trampoline(target);
        let t2 = alloc_trampoline(target);
        let t3 = alloc_trampoline(target);

        assert!(t1.is_some());
        assert!(t2.is_some());
        assert!(t3.is_some());

        // They should be different addresses
        let p1 = t1.unwrap().as_ptr();
        let p2 = t2.unwrap().as_ptr();
        let p3 = t3.unwrap().as_ptr();

        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
        assert_ne!(p1, p3);
    }
}
