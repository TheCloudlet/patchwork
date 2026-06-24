pub mod buffer;
pub mod pane;
pub mod raw_mode;
pub mod renderer;
pub mod shape;
pub mod terminal;
pub mod text;

use buffer::Buffer;
use shape::Rect;

pub trait Draw {
    /// Renders `self` into `buf`, positioned within `area`.
    ///
    /// `area` is the absolute region this drawable was assigned. A drawable's
    /// own coordinates are relative to `area`'s top-left corner: it adds
    /// `area.x` / `area.y` to place itself, and may use `area.w` / `area.h` to
    /// stay within bounds.
    fn draw(&self, buf: &mut Buffer, area: Rect);
}
