//! Regression tests for Pane composition: a Pane's declared Area is a
//! ceiling, not a promise, and no drawable can paint past it — at any
//! nesting depth. Exercises only the public interface.

use patchwork::Draw;
use patchwork::buffer::{Buffer, Color, Style};
use patchwork::pane::Pane;
use patchwork::shape::{Rect, RectShape};
use patchwork::surface::Surface;

/// A style distinguishable from the buffer's default, so a filled-but-blank
/// cell can be told apart from an untouched one.
fn marker_style() -> Style {
    Style {
        bg: Color::Indexed(1),
        ..Style::DEFAULT
    }
}

fn fill(area: Rect) -> RectShape {
    RectShape {
        area,
        style: marker_style(),
        fill: true,
    }
}

#[test]
fn sibling_bleed_a_drawable_overflowing_its_pane_leaves_the_neighbour_untouched() {
    // Two side-by-side panes, each 2 cells wide. The left pane's content
    // claims a 99-wide fill — wildly larger than its own area — but must not
    // reach into the right pane's cells.
    let mut buf = Buffer::new(1, 4);
    let mut root = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 4,
        h: 1,
    });

    let mut left = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 2,
        h: 1,
    });
    left.push(Box::new(fill(Rect {
        x: 0,
        y: 0,
        w: 99,
        h: 99,
    })));
    root.add_child(left);

    // Right pane stays empty — any bleed from the left would show up here.
    root.add_child(Pane::new(Rect {
        x: 2,
        y: 0,
        w: 2,
        h: 1,
    }));

    root.draw(&mut Surface::new(&mut buf));
    // Left pane's overflowing fill covers its own 2 cells...
    assert!((0..2).all(|x| buf.get(x, 0).unwrap().style == marker_style()));
    // ...and stops exactly there — the right pane's cells are untouched.
    assert!((2..4).all(|x| buf.get(x, 0).unwrap().style == Style::DEFAULT));
}

#[test]
fn ancestor_escape_a_grandchild_cannot_reach_past_the_grandparent() {
    // grandparent (4x4) -> parent (2x2, left half) -> grandchild claims 99x99.
    // The grandchild's fill must stop at the grandparent's 4-wide boundary.
    let mut buf = Buffer::new(4, 8);
    let mut grandparent = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 4,
        h: 4,
    });
    let mut parent = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 4,
        h: 4,
    });
    let mut grandchild = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 99,
        h: 99,
    });
    grandchild.push(Box::new(fill(Rect {
        x: 0,
        y: 0,
        w: 99,
        h: 99,
    })));
    parent.add_child(grandchild);
    grandparent.add_child(parent);

    grandparent.draw(&mut Surface::new(&mut buf));

    // Left 4 columns (the grandparent's own area) are filled; the right 4
    // are untouched — no escape past the grandparent's declared area.
    for y in 0..4 {
        assert!((0..4).all(|x| buf.get(x, y).unwrap().style == marker_style()));
        assert!((4..8).all(|x| buf.get(x, y).unwrap().style == Style::DEFAULT));
    }
}

#[test]
fn a_pane_entirely_outside_its_parent_paints_nothing() {
    let mut buf = Buffer::new(2, 2);
    let mut root = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 2,
        h: 2,
    });
    let mut offscreen = Pane::new(Rect {
        x: 10,
        y: 10,
        w: 2,
        h: 2,
    });
    offscreen.push(Box::new(fill(Rect {
        x: 0,
        y: 0,
        w: 2,
        h: 2,
    })));
    root.add_child(offscreen);

    root.draw(&mut Surface::new(&mut buf));
    assert!(buf.cells().iter().all(|c| c.ch == ' '));
}

#[test]
fn painters_algorithm_children_stack_on_top_of_own_content() {
    let mut buf = Buffer::new(1, 1);
    let mut root = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    root.push(Box::new(patchwork::shape::Dot {
        x: 0,
        y: 0,
        style: Style::DEFAULT,
    }));
    let mut child = Pane::new(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    child.push(Box::new(fill(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    })));
    root.add_child(child);

    root.draw(&mut Surface::new(&mut buf));
    // The child's fill (a blank) overwrote the parent's dot.
    assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
}
