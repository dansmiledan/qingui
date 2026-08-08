//! Shared benchmark scenes + measurement helpers. Single source of truth for
//! the host bench (`qingui/benches/time.rs` includes this via `#[path]`) and
//! the QEMU tool (`tools/qemu-time` declares `mod scenes;`).
//!
//! The render scene itself comes from `tools/qemu-mem/src/scene.rs` (the same
//! builder the memory benches use), so memory + runtime numbers stay
//! comparable.
//!
//! no_std + alloc only; the clock is injected as `now: &mut dyn FnMut() -> u64`
//! (host: ns since a base Instant; QEMU: `timer::elapsed()` SysTick ticks).

extern crate alloc;

#[path = "../../qemu-mem/src/scene.rs"]
mod mem_scene;

use alloc::format;
use alloc::vec;

use qingui::geometry::{Color, Point, Rect};
use qingui::widgets::label::LabelCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::{ObjRef, Ui};

/// How many times each primitive is drawn inside one `run_primitives` call
/// (the reported value is the per-draw average).
pub const PRIM_ITERS: u32 = 50;

pub use mem_scene::Tier;

/// A built render scene plus a leaf widget handle (for partial-dirty timing).
pub struct RenderScene {
    pub ui: Ui,
    pub leaf: ObjRef,
}

/// Scene for layout timing: a 320x240 flex container with `children` leaf
/// widgets. `layout()` is idempotent — running a pass repeatedly is
/// representative of a real relayout.
pub fn build_layout_scene(children: usize) -> Ui {
    use qingui::layout::{Align, Flex, FlexDir};
    use qingui::layout::Layout;

    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    let container = ObjCfg::new()
        .size(320, 240)
        .layout(Layout::Flex(Flex {
            dir: FlexDir::Column,
            wrap: false,
            main: Align::Start,
            cross: Align::Start,
            track: Align::Start,
            gap: 8,
        }))
        .build(&mut ui, scr);
    for i in 0..children {
        LabelCfg::new(&format!("item{i}")).size(60, 20).build(&mut ui, container);
    }
    ui.layout();
    ui
}

/// Render scene per tier: the same builder the memory benches use
/// (`tools/qemu-mem/src/scene.rs`), so both measure the same tree.
pub fn build_render_scene(tier: Tier) -> RenderScene {
    let mem_scene::Scene { ui, leaf } = mem_scene::build_scene(tier);
    RenderScene { ui, leaf }
}

/// Counts nodes and returns (nodes, width*height) for report headers.
pub fn scene_label(scene: &RenderScene) -> (usize, usize) {
    let mut n = 0;
    let mut stack = vec![scene.ui.screen()];
    while let Some(o) = stack.pop() {
        n += 1;
        stack.extend(scene.ui.children(o));
    }
    let r = scene.ui.rect(scene.ui.screen());
    (n, (r.w * r.h) as usize)
}

fn now_delta(now: &mut dyn FnMut() -> u64, f: &mut dyn FnMut()) -> u64 {
    let t0 = now();
    f();
    now() - t0
}

/// One full layout pass over the whole tree.
pub fn time_layout(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64 {
    now_delta(now, &mut || ui.layout())
}

/// Render after dirtying the full screen (worst case).
pub fn time_render_full(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64 {
    ui.invalidate_area(ui.rect(ui.screen()));
    now_delta(now, &mut || ui.render())
}

/// Render after dirtying a single leaf widget (typical interaction).
pub fn time_render_partial(ui: &mut Ui, leaf: ObjRef, now: &mut dyn FnMut() -> u64) -> u64 {
    ui.invalidate_obj(leaf);
    now_delta(now, &mut || ui.render())
}

/// End-to-end frame: anims + layout (if dirty) + floating + tick + render.
pub fn time_frame(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64 {
    ui.invalidate_area(ui.rect(ui.screen()));
    now_delta(now, &mut || {
        ui.timer_handler();
    })
}

/// Per-draw average timing for each basic primitive on a full 320x240 buffer.
pub fn run_primitives(now: &mut dyn FnMut() -> u64) -> PrimResults {
    let full = Rect::new(0, 0, 320, 240);
    let mut pixels = vec![Color::BLACK; 320 * 240];
    let mut d = qingui::draw::DrawBuf { pixels: &mut pixels, area: full, stride: 320 };
    let clip = full;
    let iters = PRIM_ITERS;

    fn bench(now: &mut dyn FnMut() -> u64, iters: u32, f: &mut dyn FnMut()) -> u64 {
        let t0 = now();
        for _ in 0..iters {
            f();
        }
        (now() - t0) / iters as u64
    }

    PrimResults {
        fill_rect: bench(now, iters, &mut || d.fill_rect(full, Color::RED, 255, clip)),
        draw_line: bench(now, iters, &mut || {
            d.draw_line(Point { x: 0, y: 0 }, Point { x: 319, y: 239 }, 2, Color::WHITE, 255, clip)
        }),
        draw_line_many: bench(now, iters, &mut || {
            for k in 0..10 {
                d.draw_line(
                    Point { x: k * 32, y: 0 },
                    Point { x: k * 32 + 16, y: 239 },
                    1,
                    Color::WHITE,
                    255,
                    clip,
                );
            }
        }),
        draw_circle: bench(now, iters, &mut || {
            d.draw_circle(Point { x: 160, y: 120 }, 60, 2, Color::WHITE, 255, clip)
        }),
        fill_circle: bench(now, iters, &mut || {
            d.fill_circle(Point { x: 160, y: 120 }, 40, Color::WHITE, 255, clip)
        }),
        fill_rounded: bench(now, iters, &mut || d.fill_rounded(full, 8, Color::WHITE, 255, clip)),
        draw_border: bench(now, iters, &mut || d.draw_border(full, 4, 8, Color::WHITE, 255, clip)),
        draw_arc: bench(now, iters, &mut || {
            d.draw_arc(Point { x: 160, y: 120 }, 80, 4, 0, 270, Color::WHITE, 255, clip)
        }),
        draw_text: bench(now, iters, &mut || {
            d.draw_text(Point { x: 10, y: 10 }, qingui::font::DEFAULT_FONT, "qingui bench", Color::WHITE, clip)
        }),
        blit565: {
            // Allocate the source image outside the timed loop: the bench
            // measures blit, not the allocator (no other primitive allocs).
            let img = vec![0u8; 32 * 24 * 2];
            bench(now, iters, &mut || d.blit565(10, 10, 32, 24, &img, 255, clip))
        },
    }
}

pub struct PrimResults {
    pub fill_rect: u64,
    pub draw_line: u64,
    pub draw_line_many: u64,
    pub draw_circle: u64,
    pub fill_circle: u64,
    pub fill_rounded: u64,
    pub draw_border: u64,
    pub draw_arc: u64,
    pub draw_text: u64,
    pub blit565: u64,
}
