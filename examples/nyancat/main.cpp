// Drives Patchwork's immediate-mode C API (src/ffi.rs) from a real C++
// program: klange/nyancat's actual animation, ported byte-for-byte (see
// frames.h), driving pw_renderer_new, pw_frame_begin, the draw primitives,
// and pw_present through RAII wrappers.
#include <fcntl.h>
#include <sys/ioctl.h>
#include <unistd.h>

#include <chrono>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <format>
#include <memory>
#include <string>
#include <thread>

#include "frames.h"
#include "patchwork.h"

namespace {

// Patchwork's C API is immediate-mode primitives only: it owns no
// terminal session or event loop, so it has no fd or signal-mask relationship
// to receive SIGWINCH against, and installing a handler inside the FFI layer
// would mean silently claiming a process-wide signal on this program's
// behalf. That puts Patchwork's C API in the same boundary position as
// libvterm (a detached component the embedder drives), not ncurses/notcurses
// (a library that owns the terminal session and installs its own handler).
// So the caller — this file — installs its own handler and reacts by
// rebuilding the renderer through the existing pw_renderer_new/
// pw_renderer_free pair; no FFI change needed.
//
// The handler only sets a flag: async-signal-safe code can't allocate or
// call back into Patchwork, so the actual resize (re-querying the terminal,
// rebuilding the renderer) happens on the next loop iteration in main().
volatile std::sig_atomic_t g_resize_pending = 0;

void handle_sigwinch(int) { g_resize_pending = 1; }

struct RendererDeleter {
  void operator()(pw_renderer_t* r) const { pw_renderer_free(r); }
};
using RendererPtr = std::unique_ptr<pw_renderer_t, RendererDeleter>;

// pw_present both flushes the frame to the terminal AND frees the handle —
// unique_ptr's deleter is exactly that "on scope exit, do the one closing
// action" shape, so wrapping it here means a frame can never be forgotten
// or double-presented without the type system complaining first.
struct FrameDeleter {
  void operator()(pw_frame_t* f) const { pw_present(f); }
};
using FramePtr = std::unique_ptr<pw_frame_t, FrameDeleter>;

// klange/nyancat draws each logical pixel as two terminal columns, to
// compensate for terminal cells being taller than wide — see its `output`
// string in nyancat.c. We do the same, so the animation's proportions match
// the original instead of looking squashed.
constexpr int kCellWidth = 2;

constexpr int kTotalTicksDefault = 200;

// Queries the real terminal size via the TIOCGWINSZ ioctl — the same
// mechanism Patchwork's own Rust-side terminal layer uses (src/terminal.rs).
// pw_frame_width/pw_frame_height report back whatever size the caller gave
// pw_renderer_new; they don't query the OS, so this has to happen before a
// renderer is even created, using a syscall Patchwork's C API has no reason
// to wrap.
//
// Queries /dev/tty directly rather than stdin/stdout: either of those can
// be redirected to a pipe or file (nyancat.c queries stdin's fd, which
// breaks the same way if stdin is ever redirected) while still running
// inside a real terminal, and /dev/tty always names the controlling
// terminal regardless of what the standard streams point at. Falls back to
// 80x24 on failure, matching nyancat.c's own default.
struct TerminalSize {
  int cols;
  int rows;
};

TerminalSize query_terminal_size() {
  TerminalSize size{80, 24};
  int tty_fd = open("/dev/tty", O_RDONLY);
  if (tty_fd < 0) return size;
  struct winsize ws;
  if (ioctl(tty_fd, TIOCGWINSZ, &ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0) {
    size.cols = ws.ws_col;
    size.rows = ws.ws_row;
  }
  close(tty_fd);
  return size;
}

void paint_cell(pw_frame_t* frame, int screen_x, int screen_y, char ch) {
  int color_index = kCatColorIndex(ch);
  if (color_index < 0) return;  // unmapped character: leave untouched
  if (screen_x < 0 || screen_y < 0) return;  // left/above the crop window
  pw_style_t style = pw_style_default();
  style.bg = pw_color_indexed(static_cast<uint8_t>(color_index));
  // Patchwork's own clipping discards anything past the right/bottom
  // edge, so an out-of-range positive coordinate here is safe to hand
  // off as-is — only negative coordinates need guarding above, since
  // pw_rect_t's fields are unsigned and would wrap.
  pw_draw_rect(frame,
               pw_rect_t{static_cast<uint16_t>(screen_x),
                         static_cast<uint16_t>(screen_y), kCellWidth, 1},
               style, true);
}

// klange/nyancat's own render loop (nyancat.c), transcribed verbatim: `x`
// and `y` are frame-relative (the cat's own 64x64 space, x=0..63); values
// outside that range are the tail (x<0) or empty background. This is
// klange/nyancat's actual code:
//
//   if (y > 23 && y < 43 && x < 0) {
//       int mod_x = ((-x+2) % 16) / 8;
//       if ((i / 2) % 2) mod_x = 1 - mod_x;
//       color = rainbow[mod_x + y-23];
//       if (color == 0) color = ',';
//   } else if (x < 0 || y < 0 || y >= FRAME_HEIGHT || x >= FRAME_WIDTH) {
//       color = ',';
//   } else {
//       color = frames[i][y][x];
//   }
char frame_color_at(int x, int y, int tick) {
  const char* const* sprite = kCatFrames[tick % kCatFrameCount];
  if (y > 23 && y < 43 && x < 0) {
    int mod_x = ((-x + 2) % 16) / 8;
    if ((tick / 2) % 2) mod_x = 1 - mod_x;
    char color = kRainbowBand[mod_x + y - 23];
    return color == '\0' ? ',' : color;
  }
  if (x < 0 || y < 0 || y >= kCatHeight || x >= kCatWidth) return ',';
  return sprite[y][x];
}

// Renders one frame cropped and centered on the terminal, exactly as
// nyancat.c does: min_col/max_col/min_row/max_row are a window of
// terminal_width/2 logical pixels (terminal_height-1 rows) centered on the
// 64x64 frame's own midpoint. A terminal smaller than the frame shows a
// centered crop of it; a terminal larger shows the whole frame surrounded
// by background — there is no minimum size requirement.
void draw_frame(pw_frame_t* frame, int tick, int terminal_cols,
                int terminal_rows) {
  int min_col = (kCatWidth - terminal_cols / kCellWidth) / 2;
  int max_col = (kCatWidth + terminal_cols / kCellWidth) / 2;
  int min_row = (kCatHeight - (terminal_rows - 1)) / 2;
  int max_row = (kCatHeight + (terminal_rows - 1)) / 2;

  for (int y = min_row; y < max_row; ++y) {
    for (int x = min_col; x < max_col; ++x) {
      char color = frame_color_at(x, y, tick);
      int screen_x = (x - min_col) * kCellWidth;
      int screen_y = y - min_row;
      paint_cell(frame, screen_x, screen_y, color);
    }
  }
}

// The "You have nyaned for N seconds!" status line, ported from
// nyancat.c's own counter display: bright white text, horizontally
// centered, integer seconds since start. nyancat.c prints this into the
// one row its own render loop deliberately leaves untouched — draw_frame
// here does the same (its row range is exactly terminal_rows - 1 tall),
// so this status line lands on that same reserved bottom row without
// competing with the animation for space.
//
// The row gets a solid fill first, then the text is drawn on top of it
// (Patchwork's pw_draw_text always overrides whatever a cell already
// holds) — so the bar is blue everywhere except where the text itself
// covers it. Uses the same palette index (17) as the scene's own
// background (kCatColorIndex(',') in frames.h) rather than the generic
// ANSI blue (index 4), so the status row matches the rest of the frame
// instead of clashing with a visibly different shade of blue.
constexpr uint8_t kBackgroundColorIndex = 17;

void draw_counter(pw_frame_t* frame, int terminal_cols, int elapsed_row,
                  double elapsed_seconds) {
  pw_style_t bar_style = pw_style_default();
  bar_style.bg = pw_color_indexed(kBackgroundColorIndex);
  pw_draw_rect(frame,
               pw_rect_t{0, static_cast<uint16_t>(elapsed_row),
                         static_cast<uint16_t>(terminal_cols), 1},
               bar_style, true);

  std::string text =
      std::format("You have nyaned for {:.0f} seconds!", elapsed_seconds);
  int len = static_cast<int>(text.size());
  int start_col = (terminal_cols - len) / 2;
  if (start_col < 0) start_col = 0;
  pw_style_t text_style = pw_style_default();
  text_style.fg =
      pw_color_indexed(15);  // bright white, matching nyancat.c's \033[1;37m
  text_style.bg = pw_color_indexed(
      kBackgroundColorIndex);  // same blue, so glyph cells blend into the bar
  pw_draw_text(frame, static_cast<uint16_t>(start_col),
               static_cast<uint16_t>(elapsed_row), static_cast<uint16_t>(len),
               1, text.c_str(), text_style);
}

}  // namespace

int main(int argc, char** argv) {
  int total_ticks = kTotalTicksDefault;
  for (int i = 1; i < argc; ++i) {
    if ((std::strcmp(argv[i], "-f") == 0 ||
         std::strcmp(argv[i], "--frames") == 0) &&
        i + 1 < argc) {
      total_ticks = std::atoi(argv[++i]);
    }
  }

  auto [terminal_cols, terminal_rows] = query_terminal_size();

  RendererPtr renderer(pw_renderer_new(static_cast<uint16_t>(terminal_rows),
                                       static_cast<uint16_t>(terminal_cols)));

  struct sigaction sa{};
  sa.sa_handler = handle_sigwinch;
  sigemptyset(&sa.sa_mask);
  sigaction(SIGWINCH, &sa, nullptr);

  auto start_time = std::chrono::steady_clock::now();

  for (int tick = 0; tick < total_ticks; ++tick) {
    if (g_resize_pending) {
      g_resize_pending = 0;
      TerminalSize size = query_terminal_size();
      terminal_cols = size.cols;
      terminal_rows = size.rows;
      // A new renderer's diff starts from an all-blank "current" buffer
      // (see Renderer::new in src/renderer.rs), but the terminal itself
      // still shows whatever was on screen before the resize. Without
      // clearing it here, the first frame after a resize only writes
      // cells that differ from blank, leaving stale glyphs from the old
      // size wherever the new frame doesn't happen to touch that cell —
      // e.g. old and new counter text overlapping into "YouYou have
      // nyaned...". \x1b[2J clears the screen, \x1b[H homes the cursor;
      // plain ANSI, not something the FFI needs to expose (same
      // caller-owns-it boundary as the SIGWINCH handler above).
      std::fputs("\x1b[2J\x1b[H", stdout);
      std::fflush(stdout);
      renderer.reset(pw_renderer_new(static_cast<uint16_t>(terminal_rows),
                                     static_cast<uint16_t>(terminal_cols)));
    }

    FramePtr frame(pw_frame_begin(renderer.get()));

    draw_frame(frame.get(), tick, terminal_cols, terminal_rows);

    // draw_frame's own row range is terminal_rows - 1 tall (see its
    // min_row/max_row comment), leaving exactly one row free — the
    // last one — for this status line, same as nyancat.c.
    std::chrono::duration<double> elapsed =
        std::chrono::steady_clock::now() - start_time;
    draw_counter(frame.get(), terminal_cols, terminal_rows - 1,
                 elapsed.count());

    // frame's deleter (pw_present) fires at end of scope.
    if (tick + 1 < total_ticks)
      std::this_thread::sleep_for(std::chrono::milliseconds(90));
  }

  return 0;
}
