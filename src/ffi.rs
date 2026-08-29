//! Immediate-mode C API, exposing primitives only (no Pane tree, no
//! callback surface for C-defined drawables).
//!
//! A C caller creates a [`Renderer`], begins a frame, narrows/ends regions to
//! move around the screen, paints primitives, and presents — the same shape
//! as an Xlib graphics context. There is nothing for C to individually free:
//! a [`Frame`] is one heap allocation, owned for exactly the span between
//! [`pw_frame_begin`] and [`pw_present`], and it is `pw_present` itself that
//! frees it.
//!
//! `Frame` holds the stack of narrowings from the root Surface down to
//! wherever drawing currently is, as plain [`Rect`]s rather than nested
//! `Surface` borrows — `Surface::sub` reborrows its parent, so a `Vec` of
//! live `Surface`s would be self-referential. Replaying the chain through
//! `Surface::sub` on every call is `O(depth)`, which is nothing next to a
//! terminal write.

use crate::Draw;
use crate::buffer::Style;
use crate::renderer::Renderer;
use crate::shape::{Dot, Line, Rect, RectShape};
use crate::surface::Surface;
use crate::text::TextBox;
use std::os::raw::c_char;

/// Opaque: owns the renderer's double buffers. Create with
/// [`pw_renderer_new`], destroy with [`pw_renderer_free`].
pub struct PwRenderer(Renderer);

impl PwRenderer {
    /// What the terminal currently shows, i.e. the frame last ended by
    /// [`pw_present`]. Not part of the C surface (there is no C-side use for
    /// a Buffer handle) — for Rust-side integration tests to assert on the
    /// rendered grid, per the FFI layer's own testing requirement.
    pub fn current(&self) -> &crate::buffer::Buffer {
        self.0.current()
    }
}

/// Opaque: the stack of narrowings for one frame. Each entry is relative to
/// the one before it — exactly the `area` passed to the `sub` that pushed it
/// — so replaying them through `Surface::sub` in order reconstructs the
/// current region. Create with [`pw_frame_begin`], consumed by
/// [`pw_present`]. Nothing else frees it.
pub struct Frame {
    renderer: *mut PwRenderer,
    stack: Vec<Rect>,
}

/// Dereferences the raw renderer pointer stashed in a [`Frame`], borrowing
/// for `'a` — the caller's choice, not tied to any borrow of the `Frame`
/// itself. A free function taking the raw pointer rather than a `Frame`
/// method: the renderer's lifetime is independent of `Frame`'s own fields
/// (it comes from the raw pointer), and a `&mut self -> &mut Renderer`
/// method would tie the two together, blocking a caller from also touching
/// `frame.stack` in the same expression.
///
/// # Safety
/// `ptr` must still be a live pointer from [`pw_renderer_new`], and `'a`
/// must not outlive that pointer's validity or alias another live borrow of
/// the same renderer.
unsafe fn renderer_of<'a>(ptr: *mut PwRenderer) -> &'a mut Renderer {
    &mut unsafe { &mut *ptr }.0
}

impl Frame {
    /// Replays the narrowing stack from the root down and runs `f` against
    /// the Surface at wherever the stack currently ends.
    ///
    /// Recursive rather than a loop, and continuation-passing rather than
    /// returning the narrowed Surface: each `Surface::sub` reborrows `&mut
    /// self` for a lifetime tied to that stack frame, so a Surface `N` levels
    /// deep can never be returned back up past level `N - 1` — the borrow
    /// chain only exists while every frame that built it is still on the
    /// (real, native) call stack. This is the same shape `Pane::draw`'s own
    /// recursion takes, just with the paint step at the bottom instead of
    /// interleaved with descent.
    fn with_surface<R>(&self, renderer: &mut Renderer, f: impl FnOnce(&mut Surface) -> R) -> R {
        fn go<R>(mut surface: Surface, remaining: &[Rect], f: impl FnOnce(&mut Surface) -> R) -> R {
            match remaining.split_first() {
                None => f(&mut surface),
                Some((area, rest)) => go(surface.sub(*area), rest, f),
            }
        }
        go(renderer.back_surface(), &self.stack, f)
    }

    /// The current region's size, found by replaying the stack.
    fn size(&self, renderer: &mut Renderer) -> (u16, u16) {
        self.with_surface(renderer, |s| (s.width(), s.height()))
    }
}

/// Creates a renderer with the given size, in cells. Free with
/// [`pw_renderer_free`].
#[unsafe(no_mangle)]
pub extern "C" fn pw_renderer_new(rows: u16, cols: u16) -> *mut PwRenderer {
    Box::into_raw(Box::new(PwRenderer(Renderer::new(rows, cols))))
}

/// Frees a renderer created by [`pw_renderer_new`].
///
/// # Safety
/// `renderer` must be a pointer returned by [`pw_renderer_new`] and not
/// already freed, and no [`Frame`] obtained from it may still be open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_renderer_free(renderer: *mut PwRenderer) {
    if !renderer.is_null() {
        drop(unsafe { Box::from_raw(renderer) });
    }
}

/// Begins a frame: blanks the back buffer and returns a handle covering it.
/// Every draw, query, `sub`/`end`, and `clear` call takes this handle; end
/// the frame with [`pw_present`], which also frees the handle.
///
/// # Safety
/// `renderer` must be a live pointer from [`pw_renderer_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_frame_begin(renderer: *mut PwRenderer) -> *mut Frame {
    // `frame()` blanks the back buffer; drop the Surface immediately since
    // the stack (empty = "the whole root") is what the rest of the API
    // replays through `back_surface` instead.
    unsafe { &mut *renderer }.0.frame();
    Box::into_raw(Box::new(Frame {
        renderer,
        stack: Vec::new(),
    }))
}

/// The width, in cells, of the region currently on top of the frame's stack.
///
/// # Safety
/// `frame` must be a live pointer from [`pw_frame_begin`], not yet passed to
/// [`pw_present`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_frame_width(frame: *mut Frame) -> u16 {
    let frame = unsafe { &*frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.size(renderer).0
}

/// The height, in cells, of the region currently on top of the frame's stack.
///
/// # Safety
/// Same as [`pw_frame_width`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_frame_height(frame: *mut Frame) -> u16 {
    let frame = unsafe { &*frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.size(renderer).1
}

/// Narrows the current region to `area`, relative to it, and pushes the
/// result onto the frame's stack — the same containment guarantee as
/// `Surface::sub`: the new region can only be smaller. Pair with
/// [`pw_surface_end`] once painting inside `area` is done.
///
/// # Safety
/// `frame` must be a live pointer from [`pw_frame_begin`], not yet passed to
/// [`pw_present`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_surface_sub(frame: *mut Frame, area: Rect) {
    unsafe { &mut *frame }.stack.push(area);
}

/// Pops the most recent [`pw_surface_sub`], returning drawing to the region
/// it narrowed from. A no-op at the root, which nothing pops below.
///
/// # Safety
/// Same as [`pw_surface_sub`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_surface_end(frame: *mut Frame) {
    unsafe { &mut *frame }.stack.pop();
}

/// Clears the current region to blank cells.
///
/// # Safety
/// `frame` must be a live pointer from [`pw_frame_begin`], not yet passed to
/// [`pw_present`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_clear(frame: *mut Frame) {
    let frame = unsafe { &mut *frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.with_surface(renderer, |s| s.clear());
}

/// Draws a single dot at `(x, y)`, relative to the current region.
///
/// # Safety
/// `frame` must be a live pointer from [`pw_frame_begin`], not yet passed to
/// [`pw_present`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_draw_dot(frame: *mut Frame, x: u16, y: u16, style: Style) {
    let frame = unsafe { &mut *frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.with_surface(renderer, |s| Dot { x, y, style }.draw(s));
}

/// Draws a line from `(x1, y1)` to `(x2, y2)`, relative to the current
/// region.
///
/// # Safety
/// Same as [`pw_draw_dot`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_draw_line(
    frame: *mut Frame,
    x1: u16,
    y1: u16,
    x2: u16,
    y2: u16,
    style: Style,
) {
    let frame = unsafe { &mut *frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.with_surface(renderer, |s| {
        Line {
            x1,
            y1,
            x2,
            y2,
            style,
        }
        .draw(s)
    });
}

/// Draws a rectangle at `area`, relative to the current region. `fill` draws
/// a solid block; otherwise just the outline.
///
/// # Safety
/// Same as [`pw_draw_dot`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_draw_rect(frame: *mut Frame, area: Rect, style: Style, fill: bool) {
    let frame = unsafe { &mut *frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.with_surface(renderer, |s| RectShape { area, style, fill }.draw(s));
}

/// Draws `text` (a NUL-terminated UTF-8 C string) at `(x, y)`, relative to
/// the current region, wrapping at `w` and truncating past `h` rows.
///
/// # Safety
/// `frame` must be as in [`pw_draw_dot`]. `text` must be a valid pointer to a
/// NUL-terminated, well-formed UTF-8 C string, live for the call's duration.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_draw_text(
    frame: *mut Frame,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    text: *const c_char,
    style: Style,
) {
    let data = unsafe { std::ffi::CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned();
    let frame = unsafe { &mut *frame };
    let renderer = unsafe { renderer_of(frame.renderer) };
    frame.with_surface(renderer, |s| {
        TextBox {
            text_area: Rect { x, y, w, h },
            data,
            style,
        }
        .draw(s)
    });
}

/// Ends the frame: diffs it against what's on screen, writes only the
/// changed cells, and frees `frame`. `frame` must not be used afterward.
///
/// # Safety
/// `frame` must be a live pointer from [`pw_frame_begin`], not already
/// passed to `pw_present`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pw_present(frame: *mut Frame) {
    let frame = unsafe { Box::from_raw(frame) };
    let renderer = unsafe { renderer_of(frame.renderer) };
    let _ = renderer.draw();
}
