// FIXME: Add clip when out-of-bound

use crate::buffer::{Buffer, Style};
use crate::Draw;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dot {
    pub x: u16,
    pub y: u16,
    pub style: Style,
}

impl Draw for Dot {
    fn draw(&self, buf: &mut Buffer, area: Rect) {
        // Coordinates are relative to `area`'s top-left corner.
        let x = area.x + self.x;
        let y = area.y + self.y;
        if let Some(cell) = buf.get_mut(x, y) {
            cell.ch = '•';
            cell.style = self.style;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Line {
    pub x1: u16,
    pub y1: u16,
    pub x2: u16,
    pub y2: u16,
    pub style: Style,
}

impl Draw for Line {
    fn draw(&self, buf: &mut Buffer, area: Rect) {
        // Bresenham's line algorithm, generalized to every octant.
        //
        // We step one cell at a time from (x1, y1) toward (x2, y2). `err`
        // tracks the accumulated distance from the ideal line; whenever it
        // crosses the threshold we also step on the minor axis. Coordinates are
        // promoted to i32 so the deltas can go negative without underflowing.
        // Endpoints are relative to `area`'s top-left corner.
        let ox = area.x as i32;
        let oy = area.y as i32;
        let mut x = ox + self.x1 as i32;
        let mut y = oy + self.y1 as i32;
        let x2 = ox + self.x2 as i32;
        let y2 = oy + self.y2 as i32;

        let dx = (x2 - x).abs();
        let dy = -(y2 - y).abs(); // negative by convention in this formulation
        let sx = if x < x2 { 1 } else { -1 };
        let sy = if y < y2 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            // Plot the current cell, skipping anything off the buffer. Negative
            // coords can't index a u16 buffer, so guard before casting.
            if x >= 0
                && y >= 0
                && let Some(cell) = buf.get_mut(x as u16, y as u16)
            {
                cell.ch = '·';
                cell.style = self.style;
            }

            if x == x2 && y == y2 {
                break;
            }

            // `e2` is twice the error; comparing it to dy/dx decides which
            // axis (or both) to advance this step.
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

/// An axis-aligned rectangle in cell coordinates: top-left corner `(x, y)`
/// with width `w` and height `h`. Pure geometry — used for layout and splits.
/// To draw a rectangle, wrap one in a [`RectShape`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    /// Splits this rect into `(top, bottom)`. `top_percent` is the top piece's
    /// share of the height, in `1..=99`. The two pieces tile the original
    /// exactly: no gap, no overlap.
    pub fn split_vertical(&self, top_percent: u16) -> (Rect, Rect) {
        debug_assert!(top_percent > 0 && top_percent < 100);

        let top_h = self.h * top_percent / 100; // multiply before dividing
        let top = Rect {
            y: self.y,
            h: top_h,
            ..*self // inherit x, w
        };
        let bottom = Rect {
            y: self.y + top_h, // starts right below the top piece
            h: self.h - top_h, // the remainder — keeps the pieces flush
            ..*self
        };
        (top, bottom)
    }

    /// Splits this rect into `(left, right)`. `left_percent` is the left
    /// piece's share of the width, in `1..=99`. The two pieces tile the
    /// original exactly: no gap, no overlap.
    pub fn split_horizontal(&self, left_percent: u16) -> (Rect, Rect) {
        debug_assert!(left_percent > 0 && left_percent < 100);

        let left_w = self.w * left_percent / 100;
        let left = Rect {
            w: left_w,
            ..*self // inherit x, y, h
        };
        let right = Rect {
            x: self.x + left_w,
            w: self.w - left_w,
            ..*self
        };
        (left, right)
    }
}

/// A drawable rectangle: a [`Rect`] area plus how to paint it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RectShape {
    pub area: Rect,
    /// Style for the rect. Border glyphs (when `fill` is false) use this style
    /// directly; when `fill` is true the interior is painted with its
    /// background. Border and fill share one style in this version.
    pub style: Style,
    /// `false` draws just the outline (`┌─┐│└┘`); `true` fills the whole area
    /// with blanks colored by `style.bg`.
    pub fill: bool,
}

impl Draw for RectShape {
    fn draw(&self, buf: &mut Buffer, area: Rect) {
        // `self.area` is relative to the given `area`'s top-left corner.
        let x = area.x + self.area.x;
        let y = area.y + self.area.y;
        let w = self.area.w;
        let h = self.area.h;

        // Empty rect: nothing to draw (also avoids underflow on w-1 / h-1).
        if w == 0 || h == 0 {
            return;
        }

        // Local helper: set the cell at absolute (x, y) to `ch` in this style.
        let style = self.style;
        let put = |buf: &mut Buffer, px: u16, py: u16, ch: char| {
            if let Some(cell) = buf.get_mut(px, py) {
                cell.ch = ch;
                cell.style = style;
            }
        };

        // Absolute edges of the rect.
        let left = x;
        let right = x + w - 1;
        let top = y;
        let bottom = y + h - 1;

        if self.fill {
            // Solid block: every cell inside the rect. Works at any size,
            // including 1xN / Nx1 strips.
            for py in top..=bottom {
                for px in left..=right {
                    put(buf, px, py, ' ');
                }
            }
            return;
        }

        // Outline needs at least 2x2 to form a box with distinct corners.
        // Thinner rects degenerate: a 1-wide rect is a vertical line, a
        // 1-tall rect is a horizontal line, and 1x1 is a single dot.
        if w == 1 && h == 1 {
            put(buf, left, top, '•');
            return;
        }
        if w == 1 {
            for py in top..=bottom {
                put(buf, left, py, '│');
            }
            return;
        }
        if h == 1 {
            for px in left..=right {
                put(buf, px, top, '─');
            }
            return;
        }

        // Outline: top/bottom edges, then left/right edges, then corners.
        for px in left..=right {
            put(buf, px, top, '─');
            put(buf, px, bottom, '─');
        }
        for py in top..=bottom {
            put(buf, left, py, '│');
            put(buf, right, py, '│');
        }
        put(buf, left, top, '┌');
        put(buf, right, top, '┐');
        put(buf, left, bottom, '└');
        put(buf, right, bottom, '┘');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Buffer, Color};

    /// Shorthand for building a geometry rect in the split tests.
    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, w, h }
    }

    /// Zero origin: drawing against it makes relative coords equal absolute,
    /// so the shape-drawing tests can keep using absolute coordinates.
    const ZERO: Rect = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    #[test]
    fn dot_draws_bullet_at_its_xy() {
        let mut buf = Buffer::new(3, 4); // 3 rows x 4 cols
        let dot = Dot {
            x: 2,
            y: 1,
            style: Style::DEFAULT,
        };
        dot.draw(&mut buf, ZERO);

        // The bullet lands at (x=2, y=1), and nowhere else.
        assert_eq!(buf.get(2, 1).unwrap().ch, '•');
        let drawn = buf.cells().iter().filter(|c| c.ch == '•').count();
        assert_eq!(drawn, 1);
    }

    #[test]
    fn dot_carries_its_style() {
        let mut buf = Buffer::new(1, 1);
        let style = Style {
            fg: Color::Indexed(9),
            ..Style::DEFAULT
        };
        Dot { x: 0, y: 0, style }.draw(&mut buf, ZERO);
        assert_eq!(buf.get(0, 0).unwrap().style, style);
    }

    #[test]
    fn dot_out_of_bounds_is_a_noop() {
        // y = 5 is past the 2 rows; drawing must not panic and must change nothing.
        let mut buf = Buffer::new(2, 2);
        Dot {
            x: 0,
            y: 5,
            style: Style::DEFAULT,
        }
        .draw(&mut buf, ZERO);
        assert!(buf.cells().iter().all(|c| c.ch != '•'));
    }

    #[test]
    fn vertical_split_tiles_exactly() {
        let r = rect(0, 0, 80, 100);
        let (top, bottom) = r.split_vertical(30);

        assert_eq!(top, rect(0, 0, 80, 30));
        assert_eq!(bottom, rect(0, 30, 80, 70));
        // Pieces are flush and cover the whole height.
        assert_eq!(top.h + bottom.h, r.h);
        assert_eq!(bottom.y, top.y + top.h);
    }

    #[test]
    fn horizontal_split_tiles_exactly() {
        let r = rect(5, 2, 100, 40);
        let (left, right) = r.split_horizontal(25);

        assert_eq!(left, rect(5, 2, 25, 40));
        assert_eq!(right, rect(30, 2, 75, 40));
        assert_eq!(left.w + right.w, r.w);
        assert_eq!(right.x, left.x + left.w);
    }

    #[test]
    fn split_preserves_origin_offset() {
        // A non-zero origin must carry through to both pieces.
        let r = rect(10, 20, 50, 50);
        let (top, bottom) = r.split_vertical(50);
        assert_eq!(top.x, 10);
        assert_eq!(top.y, 20);
        assert_eq!(bottom.x, 10);
        assert_eq!(bottom.y, 45); // 20 + 25
    }

    #[test]
    fn line_horizontal() {
        let mut buf = Buffer::new(1, 4); // 1 row, 4 cols
        Line {
            x1: 0,
            y1: 0,
            x2: 3,
            y2: 0,
            style: Style::DEFAULT,
        }
        .draw(&mut buf, ZERO);
        assert!(buf.cells().iter().all(|c| c.ch == '·'));
    }

    #[test]
    fn line_vertical() {
        let mut buf = Buffer::new(3, 1); // 3 rows, 1 col
        Line {
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 2,
            style: Style::DEFAULT,
        }
        .draw(&mut buf, ZERO);
        assert!(buf.cells().iter().all(|c| c.ch == '·'));
    }

    #[test]
    fn line_diagonal_hits_the_diagonal_cells() {
        let mut buf = Buffer::new(3, 3);
        Line {
            x1: 0,
            y1: 0,
            x2: 2,
            y2: 2,
            style: Style::DEFAULT,
        }
        .draw(&mut buf, ZERO);
        // Perfect 45°: exactly the main diagonal is drawn.
        assert_eq!(buf.get(0, 0).unwrap().ch, '·');
        assert_eq!(buf.get(1, 1).unwrap().ch, '·');
        assert_eq!(buf.get(2, 2).unwrap().ch, '·');
        // Off-diagonal corners stay blank.
        assert_eq!(buf.get(2, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(0, 2).unwrap().ch, ' ');
    }

    #[test]
    fn line_endpoints_are_inclusive_and_direction_agnostic() {
        // Drawing from B to A covers the same cells as A to B.
        let mut buf = Buffer::new(1, 3);
        Line {
            x1: 2,
            y1: 0,
            x2: 0,
            y2: 0,
            style: Style::DEFAULT,
        }
        .draw(&mut buf, ZERO);
        assert_eq!(buf.get(0, 0).unwrap().ch, '·'); // endpoint
        assert_eq!(buf.get(1, 0).unwrap().ch, '·');
        assert_eq!(buf.get(2, 0).unwrap().ch, '·'); // other endpoint
    }

    #[test]
    fn line_single_point() {
        let mut buf = Buffer::new(1, 1);
        Line {
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
            style: Style::DEFAULT,
        }
        .draw(&mut buf, ZERO);
        assert_eq!(buf.get(0, 0).unwrap().ch, '·');
    }

    #[test]
    fn rect_shape_draws_outline_with_corners() {
        let mut buf = Buffer::new(3, 3); // exactly fits a 3x3 box
        RectShape {
            area: rect(0, 0, 3, 3),
            style: Style::DEFAULT,
            fill: false,
        }
        .draw(&mut buf, ZERO);

        // Corners.
        assert_eq!(buf.get(0, 0).unwrap().ch, '┌');
        assert_eq!(buf.get(2, 0).unwrap().ch, '┐');
        assert_eq!(buf.get(0, 2).unwrap().ch, '└');
        assert_eq!(buf.get(2, 2).unwrap().ch, '┘');
        // Edges.
        assert_eq!(buf.get(1, 0).unwrap().ch, '─');
        assert_eq!(buf.get(0, 1).unwrap().ch, '│');
        // The center stays blank (outline only).
        assert_eq!(buf.get(1, 1).unwrap().ch, ' ');
    }

    #[test]
    fn rect_shape_fill_paints_every_cell() {
        let mut buf = Buffer::new(2, 2);
        let style = Style {
            bg: Color::Indexed(4),
            ..Style::DEFAULT
        };
        RectShape {
            area: rect(0, 0, 2, 2),
            style,
            fill: true,
        }
        .draw(&mut buf, ZERO);

        // Every cell is a blank painted in the fill style.
        assert!(buf.cells().iter().all(|c| c.ch == ' ' && c.style == style));
    }

    #[test]
    fn rect_shape_1xn_outline_is_a_vertical_line() {
        let mut buf = Buffer::new(3, 1); // 3 rows, 1 col
        RectShape {
            area: rect(0, 0, 1, 3),
            style: Style::DEFAULT,
            fill: false,
        }
        .draw(&mut buf, ZERO);
        // No box corners — just a vertical line of '│'.
        assert!(buf.cells().iter().all(|c| c.ch == '│'));
    }

    #[test]
    fn rect_shape_nx1_outline_is_a_horizontal_line() {
        let mut buf = Buffer::new(1, 4); // 1 row, 4 cols
        RectShape {
            area: rect(0, 0, 4, 1),
            style: Style::DEFAULT,
            fill: false,
        }
        .draw(&mut buf, ZERO);
        assert!(buf.cells().iter().all(|c| c.ch == '─'));
    }

    #[test]
    fn rect_shape_1x1_outline_is_a_dot() {
        let mut buf = Buffer::new(1, 1);
        RectShape {
            area: rect(0, 0, 1, 1),
            style: Style::DEFAULT,
            fill: false,
        }
        .draw(&mut buf, ZERO);
        assert_eq!(buf.get(0, 0).unwrap().ch, '•');
    }

    #[test]
    fn rect_shape_1xn_fill_still_paints_the_strip() {
        // Fill is unaffected by the degenerate-outline rules.
        let mut buf = Buffer::new(3, 1);
        RectShape {
            area: rect(0, 0, 1, 3),
            style: Style::DEFAULT,
            fill: true,
        }
        .draw(&mut buf, ZERO);
        assert!(buf.cells().iter().all(|c| c.ch == ' '));
    }

    #[test]
    fn integer_rounding_still_tiles() {
        // 100 * 33 / 100 = 33, remainder 67 — must still add up, no lost cell.
        let r = rect(0, 0, 10, 100);
        let (top, bottom) = r.split_vertical(33);
        assert_eq!(top.h, 33);
        assert_eq!(bottom.h, 67);
        assert_eq!(top.h + bottom.h, r.h);
    }
}
