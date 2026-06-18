use std::io;

use patchwork::buffer::{Buffer, Color, Style};
use patchwork::renderer::Renderer;
use patchwork::terminal::{Event, Key, Terminal};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;
    let size = *term.size();

    let mut renderer = Renderer::new(size.rows, size.cols);
    let mut last_key: Option<char> = None;

    // Draw the first frame, then redraw on every event.
    paint(renderer.next_mut(), last_key);
    renderer.draw()?;

    loop {
        let Some(event) = term.next_event(None)? else {
            continue;
        };

        match event {
            Event::Resize(size) => {
                // Rebuild the renderer at the new size, then repaint.
                renderer = Renderer::new(size.rows, size.cols);
            }
            Event::Key(Key::Char('q')) => break,
            Event::Key(Key::Char(c)) => last_key = Some(c),
            Event::Key(_other) => {}
        }

        paint(renderer.next_mut(), last_key);
        renderer.draw()?;
    }

    Ok(())
}

/// Paints a framed box with a title and the last key pressed.
fn paint(buf: &mut Buffer, last_key: Option<char>) {
    buf.clear();

    let rows = buf.rows();
    let cols = buf.cols();
    if rows < 3 || cols < 3 {
        return; // too small to draw a border
    }

    let border = Style {
        fg: Color::Rgb(120, 200, 255),
        bg: Color::Default,
        bold: true,
        underline: false,
    };
    draw_border(buf, border);

    let title = Style {
        fg: Color::Rgb(255, 220, 120),
        bg: Color::Default,
        bold: true,
        underline: true,
    };
    put_centered(buf, rows / 2 - 1, "✦ Patchwork ✦", title);

    let body = Style {
        fg: Color::Indexed(250),
        bg: Color::Default,
        bold: false,
        underline: false,
    };
    put_centered(buf, rows / 2 + 1, "press any key — 'q' to quit", body);

    let line = match last_key {
        Some(c) => format!("last key: {:?}", c),
        None => "last key: (none yet)".to_string(),
    };
    put_centered(buf, rows / 2 + 2, &line, body);
}

/// Draws a single-line box around the buffer's edges.
fn draw_border(buf: &mut Buffer, style: Style) {
    let rows = buf.rows();
    let cols = buf.cols();
    let last_row = rows - 1;
    let last_col = cols - 1;

    for col in 0..cols {
        put(buf, 0, col, '─', style);
        put(buf, last_row, col, '─', style);
    }
    for row in 0..rows {
        put(buf, row, 0, '│', style);
        put(buf, row, last_col, '│', style);
    }
    put(buf, 0, 0, '┌', style);
    put(buf, 0, last_col, '┐', style);
    put(buf, last_row, 0, '└', style);
    put(buf, last_row, last_col, '┘', style);
}

/// Writes `text` horizontally centered on `row`, clipped to the buffer.
fn put_centered(buf: &mut Buffer, row: u16, text: &str, style: Style) {
    let width = text.chars().count() as u16;
    let cols = buf.cols();
    let start = cols.saturating_sub(width) / 2;
    for (i, ch) in text.chars().enumerate() {
        let col = start + i as u16;
        if col >= cols {
            break;
        }
        put(buf, row, col, ch, style);
    }
}

/// Sets a single cell if it is within bounds.
fn put(buf: &mut Buffer, row: u16, col: u16, ch: char, style: Style) {
    if let Some(cell) = buf.get_mut(row, col) {
        cell.ch = ch;
        cell.style = style;
    }
}
