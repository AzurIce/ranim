//! Experimental engine-side exports for the wasm engine/plugin split.
//!
//! The engine module (root `ranim` cdylib built with the `preview` feature)
//! owns the shared `WebAssembly.Memory` and funcref table. Scene plugins are
//! separate wasm modules built with `--import-memory --import-table` that
//!
//! - allocate through [`engine_alloc`] / [`engine_dealloc`] / [`engine_realloc`]
//!   via a forwarding `#[global_allocator]`, so `Vec`/`Box` ownership works
//!   across the module boundary exactly like a native dylib, and
//! - export raw `scene_cnt()` / `get_scene(idx)` symbols (see
//!   [`crate::link_magic`]) whose returned `*const StaticScene` pointers can be
//!   handed to [`register_scene`] because both modules share one linear memory.
//!
//! Engine link flags: `--export-table --growable-table --no-stack-first
//! --global-base=67108864 --initial-memory=83886080`.

use wasm_bindgen::prelude::*;

use std::alloc::GlobalAlloc;

use crate::{Scene, link_magic::StaticScene};

// MARK: allocator exports

/// Allocate in the engine heap; called by scene plugins' forwarding allocator.
///
/// # Safety
///
/// `align` must be a power of two not exceeding 2^31.
#[unsafe(no_mangle)]
pub extern "C" fn engine_alloc(size: usize, align: usize) -> usize {
    layout_or_null(size, align, |layout| unsafe {
        std::alloc::System.alloc(layout) as usize
    })
}

/// Allocate zeroed memory in the engine heap.
///
/// # Safety
///
/// Same constraints as [`engine_alloc`].
#[unsafe(no_mangle)]
pub extern "C" fn engine_alloc_zeroed(size: usize, align: usize) -> usize {
    layout_or_null(size, align, |layout| unsafe {
        std::alloc::System.alloc_zeroed(layout) as usize
    })
}

/// Free an allocation made by [`engine_alloc`]/[`engine_realloc`].
///
/// # Safety
///
/// `ptr` must have been returned by [`engine_alloc`]/[`engine_realloc`] for an
/// allocation of `size` bytes with alignment `align`.
#[unsafe(no_mangle)]
pub extern "C" fn engine_dealloc(ptr: usize, size: usize, align: usize) {
    if ptr == 0 {
        return;
    }
    if let Ok(layout) = std::alloc::Layout::from_size_align(size, align) {
        unsafe { std::alloc::System.dealloc(ptr as *mut u8, layout) };
    }
}

/// Reallocate in the engine heap.
///
/// # Safety
///
/// `ptr` must have been allocated with (`old_size`, `align`) via the engine
/// allocator.
#[unsafe(no_mangle)]
pub extern "C" fn engine_realloc(
    ptr: usize,
    old_size: usize,
    new_size: usize,
    align: usize,
) -> usize {
    if ptr == 0 {
        return engine_alloc(new_size, align);
    }
    match std::alloc::Layout::from_size_align(old_size, align) {
        Ok(old_layout) => unsafe {
            std::alloc::System.realloc(ptr as *mut u8, old_layout, new_size) as usize
        },
        Err(_) => 0,
    }
}

fn layout_or_null(size: usize, align: usize, f: impl FnOnce(std::alloc::Layout) -> usize) -> usize {
    match std::alloc::Layout::from_size_align(size, align) {
        Ok(layout) => f(layout),
        Err(_) => 0,
    }
}

// MARK: scene registration

/// Register a scene from a plugin's `get_scene(idx)` pointer.
///
/// Returns `None` when `ptr` is null.
///
/// # Safety
///
/// `ptr` must point to a valid `StaticScene` inside the shared linear memory,
/// produced by a loaded plugin module which must stay instantiated while the
/// returned [`Scene`] (or previews built from it) is alive.
#[wasm_bindgen]
pub fn register_scene(ptr: u32) -> Option<Scene> {
    if ptr == 0 {
        return None;
    }
    // SAFETY: guaranteed by the JS loader contract, see module docs.
    let s: &StaticScene = unsafe { &*(ptr as *const StaticScene) };
    Some(Scene::from(s))
}
