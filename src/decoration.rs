use crate::Draw;
use crate::buffer::Style;
use crate::shape::{Rect, RectShape};
use crate::surface::Surface;

/// A frame drawn along the whole Surface's edge.
///
/// Carries no coordinates: it always covers `(0, 0)` to the Surface's own
/// width and height, read fresh at paint time — so the same instance stays
/// correct however large a region it's handed, including across a resize.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Border {
    pub style: Style,
}

impl Draw for Border {
    fn draw(&self, surface: &mut Surface) {
        RectShape {
            area: Rect {
                x: 0,
                y: 0,
                w: surface.width(),
                h: surface.height(),
            },
            style: self.style,
            fill: false,
        }
        .draw(surface);
    }
}

/// A background painted across the whole Surface.
///
/// Carries no coordinates, for the same reason as [`Border`]: it always
/// fills `(0, 0)` to the Surface's own width and height, read at paint time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fill {
    pub style: Style,
}

impl Draw for Fill {
    fn draw(&self, surface: &mut Surface) {
        RectShape {
            area: Rect {
                x: 0,
                y: 0,
                w: surface.width(),
                h: surface.height(),
            },
            style: self.style,
            fill: true,
        }
        .draw(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Buffer, Color};
    use crate::test_support::render;

    #[test]
    fn border_draws_a_frame_sized_to_the_surface() {
        let mut buf = Buffer::new(3, 3);
        Border {
            style: Style::DEFAULT,
        }
        .draw(&mut Surface::new(&mut buf));
        assert_eq!(render(&buf), "┌─┐\n│ │\n└─┘");
    }

    #[test]
    fn border_resizes_with_a_narrowed_sub_surface() {
        // The same Border instance, drawn into a smaller sub-region, sizes
        // itself down without being rebuilt — no dimensions stored on it.
        let mut buf = Buffer::new(4, 6);
        let mut root = Surface::new(&mut buf);
        let mut sub = root.sub(Rect {
            x: 1,
            y: 1,
            w: 3,
            h: 3,
        });
        Border {
            style: Style::DEFAULT,
        }
        .draw(&mut sub);
        assert_eq!(render(&buf), "      \n ┌─┐  \n │ │  \n └─┘  ");
    }

    #[test]
    fn border_carries_its_style() {
        let mut buf = Buffer::new(1, 1);
        let style = Style {
            fg: Color::Indexed(2),
            ..Style::DEFAULT
        };
        Border { style }.draw(&mut Surface::new(&mut buf));
        assert_eq!(buf.get(0, 0).unwrap().style, style);
    }

    #[test]
    fn fill_paints_every_cell_of_the_surface() {
        let mut buf = Buffer::new(2, 3);
        let style = Style {
            bg: Color::Indexed(4),
            ..Style::DEFAULT
        };
        Fill { style }.draw(&mut Surface::new(&mut buf));
        assert!(buf.cells().iter().all(|c| c.ch == ' ' && c.style == style));
    }

    #[test]
    fn fill_stays_within_a_narrowed_sub_surface() {
        // The fill covers columns 1..3 of row 0 only; row 1 and columns 0/3
        // are untouched. Asserted via style, since a fill's glyph is a blank
        // and can't be told apart from "untouched" by character alone.
        let marker = Style {
            bg: Color::Indexed(1),
            ..Style::DEFAULT
        };
        let mut buf = Buffer::new(2, 4);
        let mut root = Surface::new(&mut buf);
        let mut sub = root.sub(Rect {
            x: 1,
            y: 0,
            w: 2,
            h: 1,
        });
        Fill { style: marker }.draw(&mut sub);

        assert_eq!(buf.get(0, 0).unwrap().style, Style::DEFAULT);
        assert_eq!(buf.get(1, 0).unwrap().style, marker);
        assert_eq!(buf.get(2, 0).unwrap().style, marker);
        assert_eq!(buf.get(3, 0).unwrap().style, Style::DEFAULT);
        assert_eq!(buf.get(0, 1).unwrap().style, Style::DEFAULT);
    }
}
