# Patchwork

A small terminal UI toolkit for Rust, built directly on `libc`. Patchwork
manages the TTY (raw mode, the alternate screen, `SIGWINCH` resizes) and draws
to the screen with a double-buffered, diff-based renderer that only emits the
cells that actually changed.

> **Status:** 0.1.0 — early and experimental. The public API will change.

## Features

- **Raw-mode TTY session** with RAII cleanup — the terminal is always restored
  on exit, even on early return.
- **Alternate-screen** handling so the user's scrollback is left untouched.
- **Resize handling** via a self-pipe driven by a `SIGWINCH` signal handler.
- **Double-buffered renderer**: draw into a back buffer, then `draw()` diffs it
  against the front buffer and writes only the changed cells as ANSI escapes.
- **Styled cells**: 16-color, 256-color, and 24-bit truecolor, plus bold and
  underline.

## Platform support

Unix-like systems only (Linux, macOS). The TTY layer talks to `libc` directly,
so Windows is not supported.

## Running the demo

```sh
cargo run
```

This opens a framed, centered demo on the alternate screen that echoes the last
key you pressed and repaints on resize. Press `q` to quit.

## Using it

```rust
use patchwork::buffer::{Color, Style};
use patchwork::renderer::Renderer;
use patchwork::terminal::Terminal;

let mut term = Terminal::new()?;
let size = *term.size();
let mut renderer = Renderer::new(size.rows, size.cols);

// Draw into the back buffer...
let buf = renderer.next_mut();
if let Some(cell) = buf.get_mut(0, 0) {
    cell.ch = 'H';
    cell.style = Style { fg: Color::Rgb(120, 200, 255), ..Style::DEFAULT };
}

// ...then flush only the changed cells to the terminal.
renderer.draw()?;
```

## Development

```sh
cargo test            # run the unit tests
cargo clippy          # lint
cargo fmt             # format
```

## License

MIT — see [LICENSE](LICENSE).

---

Repository: <https://github.com/TheCloudlet/Patchwork>
