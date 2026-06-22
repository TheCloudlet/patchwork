use crate::buffer::{Buffer, Cell, Color, Style};
use std::fmt::Write as _;
use std::io;

#[derive(Clone, Copy)]
pub struct CellChange {
    pub row: u16,
    pub col: u16,
    pub cell: Cell,
}

#[derive(Clone)]
pub struct BufferDiff {
    pub changes: Vec<CellChange>,
}

impl BufferDiff {
    /// Whether the two buffers were identical (no cells changed).
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

pub fn diff(old: &Buffer, new: &Buffer) -> BufferDiff {
    debug_assert_eq!(old.rows(), new.rows(), "diff buffers differ in rows");
    debug_assert_eq!(old.cols(), new.cols(), "diff buffers differ in cols");

    let cols = new.cols() as usize;
    let mut changes = Vec::new();
    for (i, (new_cell, old_cell)) in new.cells().iter().zip(old.cells()).enumerate() {
        if new_cell != old_cell {
            changes.push(CellChange {
                row: (i / cols) as u16,
                col: (i % cols) as u16,
                cell: *new_cell,
            });
        }
    }
    BufferDiff { changes }
}

/// Renders a diff as a sequence of ANSI escapes into `out`.
///
/// For each changed cell: move the cursor to its position, set its style, then
/// write the character. Styles are reset (`\x1b[0m`) before each cell, so no
/// state needs to be tracked across cells.
pub fn diff_to_ansi(diff: &BufferDiff, out: &mut String) {
    for change in &diff.changes {
        // Cursor position is 1-based in ANSI; our coords are 0-based.
        let _ = write!(out, "\x1b[{};{}H", change.row + 1, change.col + 1);
        write_style(&change.cell.style, out);
        out.push(change.cell.ch);
    }
}

/// Writes the SGR escape that selects `style`, after resetting prior attributes.
fn write_style(style: &Style, out: &mut String) {
    out.push_str("\x1b[0m"); // reset, so leftover attributes don't bleed in
    if style.bold {
        out.push_str("\x1b[1m");
    }
    if style.underline {
        out.push_str("\x1b[4m");
    }
    write_color(style.fg, true, out);
    write_color(style.bg, false, out);
}

/// Writes the SGR escape for a foreground (`fg = true`) or background color.
fn write_color(color: Color, fg: bool, out: &mut String) {
    // 38 = set foreground, 48 = set background.
    let base = if fg { 38 } else { 48 };
    match color {
        Color::Default => {} // leave the terminal default
        Color::Indexed(i) => {
            let _ = write!(out, "\x1b[{};5;{}m", base, i);
        }
        Color::Rgb(r, g, b) => {
            let _ = write!(out, "\x1b[{};2;{};{};{}m", base, r, g, b);
        }
    }
}

pub struct Renderer {
    /// What the terminal currently shows.
    current: Buffer,
    /// The frame being drawn; swapped with `current` after each `draw`.
    next: Buffer,
    /// Reused ANSI scratch buffer, cleared each frame to avoid reallocating.
    ansi: String,
}

impl Renderer {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            current: Buffer::new(rows, cols),
            next: Buffer::new(rows, cols),
            ansi: String::new(),
        }
    }

    /// The buffer to draw the next frame into. Draw here, then call [`Renderer::draw`].
    pub fn next_mut(&mut self) -> &mut Buffer {
        &mut self.next
    }

    /// Diffs the next frame against the current one, emits the changed cells,
    /// then swaps the buffers so `next` becomes the new `current`.
    pub fn draw(&mut self) -> io::Result<()> {
        use std::io::Write as _;

        let changes = diff(&self.current, &self.next);

        // Nothing changed: skip the write, flush, and swap entirely.
        if changes.is_empty() {
            return Ok(());
        }

        self.ansi.clear();
        diff_to_ansi(&changes, &mut self.ansi);

        let mut stdout = io::stdout();
        stdout.write_all(self.ansi.as_bytes())?;
        stdout.flush()?;

        // Zero-copy: just exchange the two buffers' contents.
        std::mem::swap(&mut self.current, &mut self.next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_of_identical_buffers_is_empty() {
        let a = Buffer::new(2, 2);
        let b = Buffer::new(2, 2);
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn diff_reports_changed_cell_at_correct_coords() {
        let old = Buffer::new(2, 3);
        let mut new = old.clone();
        new.get_mut(2, 1).unwrap().ch = 'A'; // (x=2, y=1) => row 1, col 2

        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 1);
        assert_eq!(d.changes[0].row, 1);
        assert_eq!(d.changes[0].col, 2);
        assert_eq!(d.changes[0].cell.ch, 'A');
    }

    #[test]
    fn diff_to_ansi_moves_cursor_one_based() {
        let old = Buffer::new(1, 1);
        let mut new = old.clone();
        new.get_mut(0, 0).unwrap().ch = 'X';

        let mut out = String::new();
        diff_to_ansi(&diff(&old, &new), &mut out);
        // 0-based (0,0) -> 1-based cursor move "1;1H", then reset, then the char.
        assert!(out.contains("\x1b[1;1H"));
        assert!(out.ends_with('X'));
    }

    #[test]
    fn indexed_color_emits_256_color_escape() {
        let mut out = String::new();
        write_color(Color::Indexed(42), true, &mut out);
        assert_eq!(out, "\x1b[38;5;42m");
    }

    #[test]
    fn rgb_color_emits_truecolor_escape() {
        let mut out = String::new();
        write_color(Color::Rgb(1, 2, 3), false, &mut out); // bg
        assert_eq!(out, "\x1b[48;2;1;2;3m");
    }

    #[test]
    fn default_color_emits_nothing() {
        let mut out = String::new();
        write_color(Color::Default, true, &mut out);
        assert!(out.is_empty());
    }
}
