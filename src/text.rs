use crate::buffer::{Buffer, Style};
use crate::shape::Rect;
use crate::Draw;

/// A box of text drawn within a rectangular area.
///
/// `text_area` positions and bounds the text relative to the region passed to
/// [`Draw::draw`]: its `x`/`y` are the top-left offset, and `w`/`h` clip how
/// much is shown. This version lays out a single line — characters past `w`
/// (or any row past the first) are dropped. Wrapping and alignment can come
/// later.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TextBox {
    pub text_area: Rect,
    pub data: String,
    pub style: Style,
}

impl TextBox {
    /// A text box at `(x, y)` holding `data`, wide enough to show all of it on
    /// one line.
    pub fn new(x: u16, y: u16, data: impl Into<String>, style: Style) -> Self {
        let data = data.into();
        let w = data.chars().count() as u16;
        TextBox {
            text_area: Rect { x, y, w, h: 1 },
            data,
            style,
        }
    }
}

impl Draw for TextBox {
    fn draw(&self, buf: &mut Buffer, area: Rect) {
        // Nothing to draw if the box has no width or height.
        if self.text_area.w == 0 || self.text_area.h == 0 {
            return;
        }

        // Top-left of the text, relative to the assigned `area`.
        let ox = area.x + self.text_area.x;
        let oy = area.y + self.text_area.y;

        // Character wrap: place glyphs left to right; advance to the next row at
        // the box width or on a '\n'. Rows past `h` are dropped (truncation).
        let mut col: u16 = 0;
        let mut row: u16 = 0;
        for ch in self.data.chars() {
            if ch == '\n' {
                // Hard break: start a fresh row, leaving the rest of this one blank.
                row += 1;
                col = 0;
                if row >= self.text_area.h {
                    break;
                }
                continue;
            }
            if col >= self.text_area.w {
                // Wrap: this row is full, move down before placing the glyph.
                row += 1;
                col = 0;
                if row >= self.text_area.h {
                    break;
                }
            }
            if let Some(cell) = buf.get_mut(ox + col, oy + row) {
                cell.ch = ch;
                cell.style = self.style;
            }
            col += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Buffer, Color};

    /// Zero origin: drawing against it makes the box's own coords absolute.
    const ORIGIN: Rect = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    fn read_row(buf: &Buffer, y: u16, len: u16) -> String {
        (0..len).map(|x| buf.get(x, y).unwrap().ch).collect()
    }

    #[test]
    fn new_sizes_box_to_fit_the_text() {
        let tb = TextBox::new(2, 1, "hi", Style::DEFAULT);
        assert_eq!(
            tb.text_area,
            Rect {
                x: 2,
                y: 1,
                w: 2,
                h: 1
            }
        );
    }

    #[test]
    fn draws_characters_at_its_offset() {
        let mut buf = Buffer::new(1, 5); // 1 row, 5 cols
        TextBox::new(1, 0, "abc", Style::DEFAULT).draw(&mut buf, ORIGIN);
        // Starts at x=1: " abc " in a 5-wide row.
        assert_eq!(read_row(&buf, 0, 5), " abc ");
    }

    #[test]
    fn single_row_box_truncates_overflow() {
        let mut buf = Buffer::new(1, 6);
        // Box 3 wide, 1 tall: "hello" fills "hel", the rest has nowhere to wrap.
        let tb = TextBox {
            text_area: Rect {
                x: 0,
                y: 0,
                w: 3,
                h: 1,
            },
            data: "hello".to_string(),
            style: Style::DEFAULT,
        };
        tb.draw(&mut buf, ORIGIN);
        assert_eq!(read_row(&buf, 0, 6), "hel   ");
    }

    #[test]
    fn wraps_at_box_width() {
        let mut buf = Buffer::new(2, 4);
        // 3 wide, 2 tall: "helloo" -> "hel" / "loo".
        let tb = TextBox {
            text_area: Rect {
                x: 0,
                y: 0,
                w: 3,
                h: 2,
            },
            data: "helloo".to_string(),
            style: Style::DEFAULT,
        };
        tb.draw(&mut buf, ORIGIN);
        assert_eq!(read_row(&buf, 0, 4), "hel ");
        assert_eq!(read_row(&buf, 1, 4), "loo ");
    }

    #[test]
    fn newline_forces_a_row_break() {
        let mut buf = Buffer::new(2, 4);
        // '\n' breaks early, leaving the rest of row 0 blank.
        let tb = TextBox {
            text_area: Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 2,
            },
            data: "ab\ncd".to_string(),
            style: Style::DEFAULT,
        };
        tb.draw(&mut buf, ORIGIN);
        assert_eq!(read_row(&buf, 0, 4), "ab  ");
        assert_eq!(read_row(&buf, 1, 4), "cd  ");
    }

    #[test]
    fn drops_rows_past_box_height() {
        let mut buf = Buffer::new(3, 3);
        // 2 wide, 2 tall: "aabbcc" would be 3 rows, but the 3rd is truncated.
        let tb = TextBox {
            text_area: Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            },
            data: "aabbcc".to_string(),
            style: Style::DEFAULT,
        };
        tb.draw(&mut buf, ORIGIN);
        assert_eq!(read_row(&buf, 0, 3), "aa ");
        assert_eq!(read_row(&buf, 1, 3), "bb ");
        assert_eq!(read_row(&buf, 2, 3), "   "); // "cc" dropped
    }

    #[test]
    fn carries_its_style() {
        let mut buf = Buffer::new(1, 2);
        let style = Style {
            fg: Color::Indexed(3),
            ..Style::DEFAULT
        };
        TextBox::new(0, 0, "x", style).draw(&mut buf, ORIGIN);
        assert_eq!(buf.get(0, 0).unwrap().style, style);
    }

    #[test]
    fn area_offset_shifts_the_text() {
        // The same box drawn against a shifted area lands shifted.
        let mut buf = Buffer::new(2, 4);
        let area = Rect {
            x: 1,
            y: 1,
            w: 3,
            h: 1,
        };
        TextBox::new(0, 0, "ok", Style::DEFAULT).draw(&mut buf, area);
        assert_eq!(buf.get(1, 1).unwrap().ch, 'o');
        assert_eq!(buf.get(2, 1).unwrap().ch, 'k');
    }

    #[test]
    fn off_buffer_is_a_noop() {
        let mut buf = Buffer::new(1, 2);
        // y far below the buffer: nothing drawn, no panic.
        TextBox::new(0, 9, "zz", Style::DEFAULT).draw(&mut buf, ORIGIN);
        assert!(buf.cells().iter().all(|c| c.ch == ' '));
    }
}
