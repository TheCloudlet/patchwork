//! Regression tests for the renderer's frame lifecycle: every frame starts
//! blank, an unchanged redraw is free, and shrinking the terminal clips
//! rather than corrupts. Exercises only the public interface.

use patchwork::Draw;
use patchwork::buffer::Style;
use patchwork::renderer::Renderer;
use patchwork::shape::Dot;

fn render(buf: &patchwork::buffer::Buffer) -> String {
    (0..buf.rows())
        .map(|y| (0..buf.cols()).map(|x| buf.get(x, y).unwrap().ch).collect())
        .collect::<Vec<String>>()
        .join("\n")
}

#[test]
fn alternate_frame_ghosting_a_frame_that_skips_content_shows_it_blank() {
    let mut renderer = Renderer::new(1, 1);

    // Frame 1: paint a dot.
    let dot = Dot {
        x: 0,
        y: 0,
        style: Style::DEFAULT,
    };
    dot.draw(&mut renderer.frame());
    renderer.draw().unwrap();
    assert_eq!(renderer.current().get(0, 0).unwrap().ch, '•');

    // Frame 2: paint nothing. The double-buffer swap makes `current` the
    // frame that was `next` a moment ago — if blanking didn't happen fresh
    // each `frame()` call, this would still show frame 1's dot (the
    // "alternate-frame ghosting" bug: content survives one frame too many).
    renderer.frame();
    renderer.draw().unwrap();
    assert_eq!(renderer.current().get(0, 0).unwrap().ch, ' ');
}

#[test]
fn identical_redraw_emits_nothing() {
    let mut renderer = Renderer::new(2, 2);
    let dot = Dot {
        x: 0,
        y: 0,
        style: Style::DEFAULT,
    };

    dot.draw(&mut renderer.frame());
    renderer.draw().unwrap();
    let after_first = render(renderer.current());

    // Same content, painted again: the diff between `next` and `current` is
    // empty, so `draw` should be a no-op and the screen stays identical.
    dot.draw(&mut renderer.frame());
    renderer.draw().unwrap();
    assert_eq!(render(renderer.current()), after_first);
}

#[test]
fn shrink_resize_clips_instead_of_corrupting() {
    // A fresh, smaller renderer stands in for a shrunk terminal (this is how
    // main.rs handles a resize event: a new Renderer at the new size).
    let mut big = Renderer::new(1, 5);
    Dot {
        x: 4,
        y: 0,
        style: Style::DEFAULT,
    }
    .draw(&mut big.frame());
    big.draw().unwrap();
    assert_eq!(render(big.current()), "    •");

    let mut small = Renderer::new(1, 3);
    // The same dot, now past the 3-wide surface: clipped, not corrupting a
    // neighbouring cell or panicking.
    Dot {
        x: 4,
        y: 0,
        style: Style::DEFAULT,
    }
    .draw(&mut small.frame());
    small.draw().unwrap();
    assert_eq!(render(small.current()), "   ");
}
