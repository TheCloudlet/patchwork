// Verifies Patchwork's immediate-mode C API (src/ffi.rs) from a real C++
// program: a small flying-cat animation driving pw_renderer_new,
// pw_frame_begin, the draw primitives, and pw_present through RAII wrappers.
#include "frames.h"
#include "patchwork.h"

#include <chrono>
#include <cstdio>
#include <memory>
#include <thread>

namespace {

struct RendererDeleter {
    void operator()(pw_renderer_t *r) const { pw_renderer_free(r); }
};
using RendererPtr = std::unique_ptr<pw_renderer_t, RendererDeleter>;

// pw_present both flushes the frame to the terminal AND frees the handle —
// unique_ptr's deleter is exactly that "on scope exit, do the one closing
// action" shape, so wrapping it here means a frame can never be forgotten
// or double-presented without the type system complaining first.
struct FrameDeleter {
    void operator()(pw_frame_t *f) const { pw_present(f); }
};
using FramePtr = std::unique_ptr<pw_frame_t, FrameDeleter>;

constexpr int kRows = 12;
constexpr int kCols = 60;
constexpr int kBackgroundScrollSpeed = 1;
constexpr int kTailLength = 10;
constexpr int kTotalTicks = 80;

void draw_background(pw_frame_t *frame, int scroll) {
    // A solid backdrop first — otherwise every cell this function doesn't
    // touch is left at pw_style_default(), which shows the terminal's own
    // background instead of a painted one.
    pw_style_t black = pw_style_default();
    black.bg = pw_color_indexed(0);
    pw_draw_rect(frame, pw_rect_t{0, 0, static_cast<uint16_t>(kCols), static_cast<uint16_t>(kRows)}, black, true);

    // A sparse star field on top: every 7th column (offset by the scroll
    // position) gets a dim dot, so the whole field appears to drift left
    // over time — the cat itself never moves, only the backdrop behind it
    // does.
    pw_style_t star = pw_style_default();
    star.bg = pw_color_indexed(0);
    star.fg = pw_color_indexed(250);
    for (int x = 0; x < kCols; ++x) {
        if ((x + scroll) % 7 == 0) {
            for (int y = 0; y < kRows; y += 3) {
                pw_draw_text(frame, static_cast<uint16_t>(x), static_cast<uint16_t>(y), 1, 1, ".", star);
            }
        }
    }
}

void draw_tail(pw_frame_t *frame, int cat_x, int cat_y, int tick) {
    // The trail sits directly behind the cat's left edge. Each column's
    // color index is offset by `tick`, so the same physical cells cycle
    // through the palette over time -- a flowing-color illusion with no
    // cell ever changing position.
    int tail_row = cat_y + 1;
    for (int i = 1; i <= kTailLength; ++i) {
        int x = cat_x - i;
        if (x < 0) break;
        const uint8_t *rgb = kRainbow[(tick + i) % kRainbowCount];
        pw_style_t style = pw_style_default();
        style.bg = pw_color_rgb(rgb[0], rgb[1], rgb[2]);
        pw_draw_text(frame, static_cast<uint16_t>(x), static_cast<uint16_t>(tail_row), 1, 1, " ", style);
    }
}

void draw_cat(pw_frame_t *frame, int cat_x, int cat_y, int tick) {
    const char *const *sprite = kCatFrames[tick % kCatFrameCount];
    pw_style_t style = pw_style_default();
    style.fg = pw_color_indexed(15);
    for (int row = 0; row < kCatHeight; ++row) {
        for (int col = 0; col < kCatWidth; ++col) {
            char ch = sprite[row][col];
            if (ch == '.') continue; // transparent: let background show through
            char text[2] = {ch, '\0'};
            pw_draw_text(frame, static_cast<uint16_t>(cat_x + col), static_cast<uint16_t>(cat_y + row), 1, 1, text, style);
        }
    }
}

} // namespace

int main() {
    RendererPtr renderer(pw_renderer_new(kRows, kCols));

    const int cat_x = kCols / 2 - kCatWidth / 2;
    const int cat_y = kRows / 2 - kCatHeight / 2;

    for (int tick = 0; tick < kTotalTicks; ++tick) {
        FramePtr frame(pw_frame_begin(renderer.get()));

        draw_background(frame.get(), tick * kBackgroundScrollSpeed);
        draw_tail(frame.get(), cat_x, cat_y, tick);
        draw_cat(frame.get(), cat_x, cat_y, tick);

        // frame's deleter (pw_present) fires at end of scope.
        std::this_thread::sleep_for(std::chrono::milliseconds(90));
    }

    std::printf("\ndone: %d ticks rendered through the C API\n", kTotalTicks);
    return 0;
}
