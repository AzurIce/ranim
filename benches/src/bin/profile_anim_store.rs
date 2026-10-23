//! Per-frame evaluation, typed frame storage, and rendering profile.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use benches::test_scenes::{static_squares, transform_squares};
use ranim::{
    SceneConstructor,
    items::{
        camera::CameraFramePlugin,
        vitem::{VItemPlugin, geometry::Square},
    },
    prelude::*,
};
use ranim_core::frame::EvaluatedFrame;
use ranim_render::{
    RenderAppExt, build_render_app, extract::ExtractPlugin, resource::RenderTextures,
    utils::WgpuContext,
};

type PlacedSquare = Transformed<Square, Translation>;

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

fn profile(name: &str, iters: u64, mut f: impl FnMut()) {
    f();
    take_allocs();
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    let elapsed = start.elapsed();
    let (allocs, bytes) = take_allocs();
    println!(
        "{name:<36} iters={iters:<4} total={elapsed:>9.2?} per-iter={:>9.2?} allocs={:>6} alloc_bytes={:>9}",
        elapsed / iters as u32,
        allocs / iters,
        bytes / iters,
    );
}

fn main() {
    let n = 20;
    let cpu_only = std::env::var_os("RANIM_PROFILE_CPU_ONLY").is_some();
    let cpu_iters = if cpu_only { 10_000 } else { 100 };
    let scene = (|r: &mut RanimScene| static_squares(r, n)).build_scene();

    profile("eval_at_alpha (400 items)", cpu_iters, || {
        let mut frame = EvaluatedFrame::new();
        scene.eval_at_alpha(0.5, &mut frame);
        std::hint::black_box(frame);
    });

    let mut store = EvaluatedFrame::new();
    profile("eval into reused frame", cpu_iters, || {
        scene.eval_at_alpha(0.5, &mut store);
    });

    if cpu_only {
        return;
    }

    let ctx = pollster::block_on(WgpuContext::new());
    let mut renderer = build_render_app(&ctx, 1920, 1080, 8);
    renderer.add_plugins((
        CameraFramePlugin,
        ExtractPlugin::<PlacedSquare>::default(),
        VItemPlugin,
    ));
    renderer.finish();
    renderer.cleanup();
    let mut render_textures = RenderTextures::new(&ctx, 1920, 1080);
    let clear_color = wgpu::Color::BLACK;

    let cold = Instant::now();
    renderer.render_frame(&mut render_textures, clear_color, &mut store);
    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    println!(
        "{:<36} time={:>9.2?}",
        "static frame (cold)",
        cold.elapsed()
    );

    profile("static frame (steady)", 20, || {
        renderer.render_frame(&mut render_textures, clear_color, &mut store);
    });

    let animated = (|r: &mut RanimScene| transform_squares(r, n)).build_scene();
    let mut animated_store = EvaluatedFrame::new();
    let frames = 30;
    let mut frame = 0;
    profile("animated frame (eval+update+render)", frames, || {
        let alpha = frame as f64 / frames as f64;
        frame += 1;
        animated.eval_at_alpha(alpha, &mut animated_store);
        renderer.render_frame(&mut render_textures, clear_color, &mut animated_store);
    });

    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
}
