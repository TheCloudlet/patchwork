pub mod buffer;
pub mod pane;
pub mod raw_mode;
pub mod renderer;
pub mod shape;
pub mod surface;
pub mod terminal;
pub mod text;

use surface::Surface;

pub trait Draw {
    /// Renders `self` onto `surface`.
    ///
    /// A drawable's own coordinates are relative to the surface's own
    /// top-left corner. Anything written outside the surface is discarded —
    /// clipping is the surface's job, not the drawable's.
    fn draw(&self, surface: &mut Surface);
}

/// Test-only helpers shared across modules' `#[cfg(test)]` blocks.
#[cfg(test)]
pub(crate) mod test_support {
    use crate::buffer::Buffer;

    /// Renders a buffer as one string per row, joined by newlines — the whole
    /// grid in one assertion, and a readable picture when it fails.
    pub(crate) fn render(buf: &Buffer) -> String {
        (0..buf.rows())
            .map(|y| (0..buf.cols()).map(|x| buf.get(x, y).unwrap().ch).collect())
            .collect::<Vec<String>>()
            .join("\n")
    }
}
