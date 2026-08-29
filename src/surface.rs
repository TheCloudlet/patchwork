use crate::buffer::{Buffer, Cell, Style};
use crate::shape::Rect;

/// A [`Buffer`] restricted to one rectangular region.
///
/// A Surface is the only way to paint. It holds no cells of its own — it is a
/// borrow of the buffer plus the viewport it is allowed to touch — so creating
/// one is just a pointer and four numbers.
///
/// Coordinates are **relative to the surface's own top-left corner**: a
/// drawable writes to `(0, 0)` to hit its own corner, wherever that lands on
/// screen. Writes outside the viewport are discarded, so clipping is a property
/// of the Surface rather than a rule each drawable has to remember.
pub struct Surface<'a> {
    buf: &'a mut Buffer,
    /// The region this surface may paint, in absolute buffer coordinates.
    viewport: Rect,
}

impl<'a> Surface<'a> {
    /// A surface covering the whole buffer.
    pub fn new(buf: &'a mut Buffer) -> Self {
        let viewport = Rect {
            x: 0,
            y: 0,
            w: buf.cols(),
            h: buf.rows(),
        };
        Surface { buf, viewport }
    }

    /// The width of this surface, in cells.
    pub fn width(&self) -> u16 {
        self.viewport.w
    }

    /// The height of this surface, in cells.
    pub fn height(&self) -> u16 {
        self.viewport.h
    }

    /// Translates a relative coordinate to an absolute one, or `None` if it
    /// falls outside the viewport. This single check is what enforces clipping.
    fn absolute(&self, x: u16, y: u16) -> Option<(u16, u16)> {
        if x >= self.viewport.w || y >= self.viewport.h {
            return None;
        }
        Some((self.viewport.x + x, self.viewport.y + y))
    }

    /// Paints one cell at `(x, y)`, relative to this surface's corner.
    /// Coordinates outside the surface are silently dropped.
    pub fn set(&mut self, x: u16, y: u16, ch: char, style: Style) {
        let Some((ax, ay)) = self.absolute(x, y) else {
            return;
        };
        if let Some(cell) = self.buf.get_mut(ax, ay) {
            cell.ch = ch;
            cell.style = style;
        }
    }

    /// Reads the cell at `(x, y)`, relative to this surface's corner.
    /// Mainly for tests — drawing never needs to read.
    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        let (ax, ay) = self.absolute(x, y)?;
        self.buf.get(ax, ay)
    }

    /// Resets every cell in this surface to the default blank cell. Only this
    /// surface's region is touched.
    pub fn clear(&mut self) {
        for y in 0..self.viewport.h {
            for x in 0..self.viewport.w {
                self.set(x, y, Cell::DEFAULT.ch, Cell::DEFAULT.style);
            }
        }
    }

    /// Narrows this surface to `area`, expressed relative to this surface's
    /// corner.
    ///
    /// The result is intersected with the current viewport, so it can only ever
    /// be smaller: a request reaching past the edge is trimmed, and one that
    /// misses entirely yields an empty surface that discards every write. This
    /// is what stops a descendant from painting outside its ancestors.
    ///
    /// The sub-surface borrows this one, so only one may be live at a time —
    /// paint one region, drop it, then move to the next.
    pub fn sub(&mut self, area: Rect) -> Surface<'_> {
        // Promote the request into absolute coordinates before intersecting;
        // saturating so an offset past the buffer clamps instead of wrapping.
        let requested = Rect {
            x: self.viewport.x.saturating_add(area.x),
            y: self.viewport.y.saturating_add(area.y),
            w: area.w,
            h: area.h,
        };
        Surface {
            viewport: requested.intersect(self.viewport),
            buf: self.buf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Color;
    use crate::test_support::render;

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn new_surface_covers_the_whole_buffer() {
        let mut buf = Buffer::new(3, 5); // 3 rows, 5 cols
        let s = Surface::new(&mut buf);
        assert_eq!(s.width(), 5);
        assert_eq!(s.height(), 3);
    }

    #[test]
    fn set_uses_relative_coordinates() {
        let mut buf = Buffer::new(2, 4);
        let mut s = Surface::new(&mut buf);
        let mut sub = s.sub(rect(1, 1, 2, 1));
        // (0,0) on the sub-surface is (1,1) on the buffer.
        sub.set(0, 0, 'x', Style::DEFAULT);
        assert_eq!(render(&buf), "    \n x  ");
    }

    #[test]
    fn writes_outside_the_surface_are_dropped() {
        let mut buf = Buffer::new(2, 4);
        let mut s = Surface::new(&mut buf);
        let mut sub = s.sub(rect(0, 0, 2, 1));
        sub.set(2, 0, 'x', Style::DEFAULT); // past the width
        sub.set(0, 1, 'y', Style::DEFAULT); // past the height
        assert_eq!(render(&buf), "    \n    ");
    }

    #[test]
    fn set_carries_its_style() {
        let mut buf = Buffer::new(1, 1);
        let style = Style {
            fg: Color::Indexed(5),
            ..Style::DEFAULT
        };
        Surface::new(&mut buf).set(0, 0, 'q', style);
        assert_eq!(buf.get(0, 0).unwrap().style, style);
    }

    #[test]
    fn sub_reports_its_own_size() {
        let mut buf = Buffer::new(10, 10);
        let mut s = Surface::new(&mut buf);
        let sub = s.sub(rect(2, 3, 4, 5));
        assert_eq!((sub.width(), sub.height()), (4, 5));
    }

    #[test]
    fn sub_is_clamped_to_its_parent() {
        // Asking for more than the parent has yields only what the parent has.
        let mut buf = Buffer::new(10, 10);
        let mut s = Surface::new(&mut buf);
        let mut half = s.sub(rect(0, 0, 4, 4));
        let greedy = half.sub(rect(2, 2, 100, 100));
        assert_eq!((greedy.width(), greedy.height()), (2, 2));
    }

    #[test]
    fn sub_outside_the_parent_is_empty() {
        let mut buf = Buffer::new(4, 4);
        let mut s = Surface::new(&mut buf);
        let mut small = s.sub(rect(0, 0, 2, 2));
        let mut miss = small.sub(rect(5, 5, 2, 2));
        assert_eq!((miss.width(), miss.height()), (0, 0));
        miss.set(0, 0, 'x', Style::DEFAULT); // must not panic, must not paint
        assert_eq!(render(&buf), "    \n    \n    \n    ");
    }

    #[test]
    fn a_descendant_cannot_escape_its_ancestor() {
        // The regression this whole design exists to prevent: a grandchild
        // asking for a huge area must not reach past the grandparent.
        let mut buf = Buffer::new(4, 8);
        let mut root = Surface::new(&mut buf);
        {
            let mut left = root.sub(rect(0, 0, 4, 4)); // left half only
            let mut inner = left.sub(rect(0, 0, 99, 99)); // wants everything
            for y in 0..inner.height() {
                for x in 0..inner.width() {
                    inner.set(x, y, '#', Style::DEFAULT);
                }
            }
        }
        // The right half is untouched — no bleed.
        assert_eq!(render(&buf), "####    \n####    \n####    \n####    ");
    }

    #[test]
    fn clear_only_resets_its_own_region() {
        let mut buf = Buffer::new(2, 4);
        {
            let mut s = Surface::new(&mut buf);
            for y in 0..2 {
                for x in 0..4 {
                    s.set(x, y, '#', Style::DEFAULT);
                }
            }
        }
        {
            let mut s = Surface::new(&mut buf);
            let mut sub = s.sub(rect(1, 0, 2, 1));
            sub.clear();
        }
        assert_eq!(render(&buf), "#  #\n####");
    }

    #[test]
    fn get_reads_back_relative() {
        let mut buf = Buffer::new(3, 3);
        let mut s = Surface::new(&mut buf);
        let mut sub = s.sub(rect(1, 1, 2, 2));
        sub.set(1, 1, 'z', Style::DEFAULT);
        assert_eq!(sub.get(1, 1).unwrap().ch, 'z');
        assert!(sub.get(2, 0).is_none()); // outside the sub-surface
    }

    #[test]
    fn sub_of_zero_sized_area_paints_nothing() {
        let mut buf = Buffer::new(2, 2);
        let mut s = Surface::new(&mut buf);
        let mut empty = s.sub(rect(0, 0, 0, 0));
        empty.set(0, 0, 'x', Style::DEFAULT);
        assert_eq!(render(&buf), "  \n  ");
    }
}
