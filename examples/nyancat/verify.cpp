// Smoke test for Patchwork's immediate-mode C API (src/ffi.rs), run from a
// real C++ toolchain rather than Rust's tests/ffi.rs. The C API has no
// read-back capability — no ownable objects cross the boundary, and no
// query primitive either — so this can only assert non-crashing,
// correct-exit-code behavior — never that a specific cell ends up a
// specific color. It knows nothing about any particular application built
// on the API; it exists purely to catch a regression that crashes or hangs
// a C/C++ caller before that caller's own visual output has to be
// inspected by hand.
#include "patchwork.h"

#include <cassert>
#include <cstdio>
#include <memory>

namespace {

struct RendererDeleter {
    void operator()(pw_renderer_t *r) const { pw_renderer_free(r); }
};
using RendererPtr = std::unique_ptr<pw_renderer_t, RendererDeleter>;

struct FrameDeleter {
    void operator()(pw_frame_t *f) const { pw_present(f); }
};
using FramePtr = std::unique_ptr<pw_frame_t, FrameDeleter>;

} // namespace

int main() {
    constexpr uint16_t kRows = 4;
    constexpr uint16_t kCols = 8;

    RendererPtr renderer(pw_renderer_new(kRows, kCols));
    assert(renderer != nullptr);

    FramePtr frame(pw_frame_begin(renderer.get()));
    assert(frame != nullptr);
    assert(pw_frame_width(frame.get()) == kCols);
    assert(pw_frame_height(frame.get()) == kRows);

    // Exercise every exported primitive once, so a crash or hang in any of
    // them fails this program directly instead of only ever showing up as
    // a garbled screen some other caller has to notice by eye.
    pw_style_t style = pw_style_default();
    pw_draw_rect(frame.get(), pw_rect_t{0, 0, kCols, kRows}, style, true);
    pw_draw_dot(frame.get(), 0, 0, style);
    pw_draw_line(frame.get(), 0, 0, kCols - 1, 0, style);
    pw_draw_text(frame.get(), 0, 1, 3, 1, "hi", style);
    pw_surface_sub(frame.get(), pw_rect_t{1, 1, 2, 2});
    pw_clear(frame.get());
    pw_surface_end(frame.get());

    // frame's and renderer's destructors run here (pw_present, then
    // pw_renderer_free) — reaching this line without a crash is the test.
    std::puts("OK: FFI smoke test passed");
    return 0;
}
