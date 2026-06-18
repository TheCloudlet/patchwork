/// A foreground or background color.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// The terminal's configured default color.
    Default,
    /// A palette index: 0-15 are the ANSI 16 colors, 16-255 the 256-color cube.
    Indexed(u8),
    /// 24-bit truecolor, as `(r, g, b)`.
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Foreground (text) color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    pub bold: bool,
    pub underline: bool,
}

impl Style {
    pub const DEFAULT: Self = Self {
        fg: Color::Default,
        bg: Color::Default,
        bold: false,
        underline: false,
    };
}

/// A single terminal cell: one character plus its styling.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// The character drawn in this cell.
    pub ch: char,
    pub style: Style,
}

impl Cell {
    pub const DEFAULT: Self = Self {
        ch: ' ',
        style: Style::DEFAULT,
    };
}

#[derive(Clone)]
pub struct Buffer {
    rows: u16,
    cols: u16,
    cells: Vec<Cell>, // Length = rows * cols
}

impl Buffer {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            cells: vec![Cell::DEFAULT; rows as usize * cols as usize],
        }
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::DEFAULT);
    }

    fn offset(&self, row: u16, col: u16) -> Option<usize> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        Some(row as usize * self.cols as usize + col as usize)
    }

    /// The backing cells in row-major order (length `rows * cols`).
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn get(&self, row: u16, col: u16) -> Option<&Cell> {
        self.offset(row, col).map(|i| &self.cells[i])
    }

    pub fn get_mut(&mut self, row: u16, col: u16) -> Option<&mut Cell> {
        self.offset(row, col).map(|i| &mut self.cells[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_all_default_cells() {
        let buf = Buffer::new(2, 3);
        assert_eq!(buf.rows(), 2);
        assert_eq!(buf.cols(), 3);
        assert_eq!(buf.cells().len(), 6);
        assert!(buf.cells().iter().all(|c| *c == Cell::DEFAULT));
    }

    #[test]
    fn get_returns_none_out_of_bounds() {
        let buf = Buffer::new(2, 2);
        assert!(buf.get(0, 0).is_some());
        assert!(buf.get(2, 0).is_none()); // row out of range
        assert!(buf.get(0, 2).is_none()); // col out of range
    }

    #[test]
    fn get_mut_writes_are_visible() {
        let mut buf = Buffer::new(1, 1);
        buf.get_mut(0, 0).unwrap().ch = 'x';
        assert_eq!(buf.get(0, 0).unwrap().ch, 'x');
    }

    #[test]
    fn clear_resets_every_cell() {
        let mut buf = Buffer::new(1, 2);
        buf.get_mut(0, 1).unwrap().ch = 'z';
        buf.clear();
        assert!(buf.cells().iter().all(|c| *c == Cell::DEFAULT));
    }
}
