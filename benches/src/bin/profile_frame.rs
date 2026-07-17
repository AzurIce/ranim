//! Per-frame profiling for the ECS migration baseline.
//!
//! Reports wall time, allocation counts/bytes and GPU upload volume for the
//! main per-frame paths: timeline eval, main-world reconciliation and render
//! submission (extract + queue + prepare + encode).
//!
//! Run with: `cargo run -p benches --bin profile_frame --release`

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use benches::test_scenes::{static_squares, transform_squares};
use ranim::{SceneConstructor, prelude::*};
use ranim_core::store::CoreItemStore;
use ranim_render::{Renderer, resource::RenderPool, utils::WgpuContext, utils::take_uploaded_bytes};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn take_allocs() -> (u64, u64) {
    (
        ALLOC_COUNT.swap(0, Ordering::Relaxed),
        ALLOC_BYTES.swap(0, Ordering::Relaxed),
    )
}

struct Section {
    name: &'static str,
    iters: u64,
    time: Duration,
    allocs: u64,
    alloc_bytes: u64,
}

impl Section {
    fn print(&self) {
        println!(
            "{:<28} iters={:<4} total={:>9.2?}  per-iter: {:>9.2?}  allocs={:>6}  alloc_bytes={:>9}",
            self.name,
            self.iters,
            self.time,
            self.time / self.iters as u32,
            self.allocs / self.iters,
            self.alloc_bytes / self.iters,
        );
    }
}

fn profile(name: &'static str, iters: u64, mut f: impl FnMut()) -> Section {
    // Warm up, then reset the counters so one-time setup cost is excluded.
    f();
    take_allocs();
    let time = Instant::now();
    for _ in 0..iters {
        f();
    }
    let time = time.elapsed();
    let (allocs, alloc_bytes) = take_allocs();
    Section {
        name,
        iters,
        time,
        allocs,
        alloc_bytes,
    }
}

fn main() {
    let n = 20; // 20x20 = 400 vitems
    let scene = (|r: &mut RanimScene| static_squares(r, n)).build_scene();

    // -- CPU: timeline evaluation -------------------------------------------
    profile("eval_at_alpha (400 items)", 100, || {
        std::hint::black_box(scene.eval_at_alpha(0.5).collect::<Vec<_>>());
    })
    .print();

    // -- CPU: eval + main-world reconciliation -------------------------------
    let mut store = CoreItemStore::new();
    profile("eval + store.update", 100, || {
        store.update(scene.eval_at_alpha(0.5));
    })
    .print();

    // -- GPU: render submission on a static frame ----------------------------
    let ctx = pollster::block_on(WgpuContext::new());
    let mut renderer = Renderer::new(&ctx, 1920, 1080, 8);
    let mut render_textures = renderer.new_render_textures(&ctx);
    let mut pool = RenderPool::new();
    let clear_color = wgpu::Color::BLACK;

    // First (cold) frame: allocates GPU resources and uploads everything.
    take_uploaded_bytes();
    let cold = Instant::now();
    renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &mut store, &mut pool);
    pool.clean();
    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    let cold_time = cold.elapsed();
    let cold_upload = take_uploaded_bytes();
    println!(
        "{:<28} time={:>9.2?}  upload={:>9} bytes",
        "static frame (cold)", cold_time, cold_upload
    );

    // Steady frames on identical content: uploads should be skipped.
    let mut steady_upload_max = 0u64;
    let steady = profile("static frame (steady)", 20, || {
        take_uploaded_bytes();
        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &mut store, &mut pool);
        pool.clean();
        steady_upload_max = steady_upload_max.max(take_uploaded_bytes());
    });
    steady.print();
    println!(
        "{:<28} max upload per steady frame: {} bytes",
        "static frame (steady)", steady_upload_max
    );

    // -- Animated frames: eval + update + render each frame ------------------
    let anim_scene = (|r: &mut RanimScene| transform_squares(r, n)).build_scene();
    let mut anim_store = CoreItemStore::new();
    let frames = 30u64;
    let mut frame = 0u64;
    let animated = profile("animated frame (eval+update+render)", frames, || {
        let alpha = frame as f64 / frames as f64;
        frame += 1;
        anim_store.update(anim_scene.eval_at_alpha(alpha));
        take_uploaded_bytes();
        renderer.render_store_with_pool(
            &ctx,
            &mut render_textures,
            clear_color,
            &mut anim_store,
            &mut pool,
        );
        pool.clean();
        std::hint::black_box(take_uploaded_bytes());
    });
    animated.print();

    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
}
