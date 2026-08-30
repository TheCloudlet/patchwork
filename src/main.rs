use std::io;

use patchwork::Draw;
use patchwork::buffer::{Color, Style};
use patchwork::decoration::{Border, Fill};
use patchwork::pane::Pane;
use patchwork::renderer::Renderer;
use patchwork::shape::{Dot, Line, Rect};
use patchwork::terminal::{Event, Key, Terminal};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;
    let size = *term.size();

    let mut renderer = Renderer::new(size.rows, size.cols);

    // Draw the first frame, then redraw on every event.
    paint(&mut renderer);
    renderer.draw()?;

    loop {
        let Some(event) = term.next_event(None)? else {
            continue;
        };

        match event {
            Event::Resize(size) => {
                renderer = Renderer::new(size.rows, size.cols);
            }
            Event::Key(Key::Char('q')) => break,
            Event::Key(_other) => {}
        }

        paint(&mut renderer);
        renderer.draw()?;
    }

    Ok(())
}

/// Builds the four-quadrant demo pane tree and draws it into the next frame.
///
/// Layout: the screen is split top/bottom, then each half left/right, giving
/// four quadrants. Each shows one drawable kind:
///   Q1 (top-left)     a single Dot
///   Q2 (top-right)    a diagonal Line
///   Q3 (bottom-left)  a Fill inset inside the border (a "face"/surface)
///   Q4 (bottom-right) a recursive Pane, itself split into two outlined boxes
fn paint(renderer: &mut Renderer) {
    let mut surface = renderer.frame();

    let cols = surface.width();
    let rows = surface.height();
    if cols < 4 || rows < 4 {
        return; // too small to be worth splitting
    }

    let screen = Rect {
        x: 0,
        y: 0,
        w: cols,
        h: rows,
    };
    let (top, bottom) = screen.split_vertical(50);
    let (q1, q2) = top.split_horizontal(50);
    let (q3, q4) = bottom.split_horizontal(50);

    // Each quadrant is a child pane. Its `area` positions it; the drawables
    // inside use coordinates relative to that pane's top-left corner.
    let mut root = Pane::new(screen);
    root.add_child(quadrant_dot(q1));
    root.add_child(quadrant_line(q2));
    root.add_child(quadrant_face(q3));
    root.add_child(quadrant_recursive(q4));

    root.draw(&mut surface);
}

/// A border around whatever Surface the pane it's pushed onto is handed —
/// no dimensions to pass in or keep in sync.
fn border(color: Color) -> Border {
    Border {
        style: solid(color),
    }
}

/// Q1: a frame plus a single Dot at the quadrant's center.
fn quadrant_dot(area: Rect) -> Pane {
    let mut pane = Pane::new(area);
    pane.push(Box::new(border(Color::Rgb(120, 200, 255))));
    pane.push(Box::new(Dot {
        x: area.w / 2,
        y: area.h / 2,
        style: solid(Color::Rgb(255, 90, 90)),
    }));
    pane
}

/// Q2: a frame plus a diagonal Line across the quadrant's interior.
fn quadrant_line(area: Rect) -> Pane {
    let mut pane = Pane::new(area);
    pane.push(Box::new(border(Color::Rgb(120, 200, 255))));
    pane.push(Box::new(Line {
        x1: 1,
        y1: 1,
        x2: area.w.saturating_sub(2),
        y2: area.h.saturating_sub(2),
        style: solid(Color::Rgb(120, 255, 120)),
    }));
    pane
}

/// Q3: a frame around a Fill — a solid "surface" inset inside the border.
fn quadrant_face(area: Rect) -> Pane {
    let mut pane = Pane::new(area);
    pane.push(Box::new(border(Color::Rgb(120, 200, 255))));
    if area.w > 2 && area.h > 2 {
        let mut inset = Pane::new(Rect {
            x: 1,
            y: 1,
            w: area.w - 2,
            h: area.h - 2,
        });
        inset.push(Box::new(Fill {
            style: Style {
                bg: Color::Rgb(60, 60, 120),
                ..Style::DEFAULT
            },
        }));
        pane.add_child(inset);
    }
    pane
}

/// Q4: a recursive Pane. The quadrant is itself split into two sub-panes,
/// each drawing its own outlined box — a pane tree nested inside a pane.
fn quadrant_recursive(area: Rect) -> Pane {
    let mut pane = Pane::new(area);
    pane.push(Box::new(border(Color::Rgb(120, 200, 255))));

    // Split the quadrant's interior (in local coords) into left/right sub-panes.
    if area.w > 4 && area.h > 2 {
        let inner = Rect {
            x: 1,
            y: 1,
            w: area.w - 2,
            h: area.h - 2,
        };
        let (left, right) = inner.split_horizontal(50);

        let mut left_pane = Pane::new(left);
        left_pane.push(Box::new(border(Color::Rgb(255, 220, 120))));

        let mut right_pane = Pane::new(right);
        right_pane.push(Box::new(border(Color::Rgb(255, 140, 220))));

        pane.add_child(left_pane);
        pane.add_child(right_pane);
    }
    pane
}

/// A plain style with the given foreground color and defaults elsewhere.
fn solid(fg: Color) -> Style {
    Style {
        fg,
        ..Style::DEFAULT
    }
}
