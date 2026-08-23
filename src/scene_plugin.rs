//! Client-side runtime for wasm scene plugins (feature `scene-plugin`).
//!
//! A scene plugin is a wasm cdylib built against the shared engine module
//! (`ranim` with the `preview` feature): it imports the engine's linear
//! memory and funcref table, and forwards all heap allocation to the
//! engine's exports, so `Vec`/`Box` values created by scene code can be
//! owned and dropped by the engine exactly like with a native dylib.
//!
//! Plugin link flags (applied by the build toolchain):
//! `--import-memory --import-table --no-stack-first
//! --global-base=16777216 --table-base=65536`
//!
//! On non-wasm targets this module compiles to nothing: native dylibs share
//! the host process allocator, so no forwarding is needed.

use std::alloc::{GlobalAlloc, Layout};

// SAFETY: these resolve to exports of the engine module provided by the JS
// loader at instantiation time.
#[link(wasm_import_module = "engine")]
unsafe extern "C" {
    fn engine_alloc(size: usize, align: usize) -> usize;
    fn engine_alloc_zeroed(size: usize, align: usize) -> usize;
    fn engine_dealloc(ptr: usize, size: usize, align: usize);
    fn engine_realloc(ptr: usize, old_size: usize, new_size: usize, align: usize) -> usize;
}

/// Forwards every allocation to the engine module's exports.
struct EngineAllocator;

unsafe impl GlobalAlloc for EngineAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: caller guarantees a valid layout; engine side validates again.
        unsafe { engine_alloc(layout.size(), layout.align()) as *mut u8 }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: see `alloc`.
        unsafe { engine_alloc_zeroed(layout.size(), layout.align()) as *mut u8 }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        // SAFETY: ptr was allocated via the engine allocator with this layout.
        unsafe { engine_dealloc(ptr as usize, layout.size(), layout.align()) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            // SAFETY: valid layout by contract.
            return unsafe {
                self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()))
            };
        }
        // SAFETY: ptr was allocated via the engine allocator with this layout.
        unsafe { engine_realloc(ptr as usize, layout.size(), new_size, layout.align()) as *mut u8 }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: EngineAllocator = EngineAllocator;
