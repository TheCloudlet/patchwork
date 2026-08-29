# Drawables paint onto a Surface, not a `&mut Buffer`

`Draw::draw` used to receive `&mut Buffer` plus a `Rect` origin, which meant
every drawable was free to write any cell on the screen and had to remember to
stay inside its own region. Nobody did: all four drawables clipped against the
buffer instead of their assigned area, `shape.rs` carried a `FIXME` about it,
and a child Pane could silently paint over its siblings. We now hand drawables a
`Surface` — a Buffer restricted to one viewport — so that writes outside the
region are impossible rather than merely discouraged.

## Considered Options

- **Each drawable clips itself.** Rejected: this is an invariant enforced by
  discipline, and the evidence is that we already failed at it in all four
  implementations. It is also the most code, since correct clipping for
  Bresenham lines is genuinely fiddly and would be rewritten per drawable.
- **Let drawables overpaint, then restore.** Rejected: requires saving the
  region before every draw, and the restore step interacts badly with siblings
  already painted.

## Consequences

- Breaking change to the public `Draw` trait. Acceptable at 0.1.0, which already
  declares the API unstable.
- Clipping is enforced in exactly one place (`Surface::set`), so the drawables
  got *smaller*: each lost its manual `area.x + self.x` translation, and the
  `ORIGIN`/`ZERO` sentinel rects disappeared entirely.
- `Buffer::get_mut` is now `pub(crate)` and `Renderer::next_mut` is replaced by
  `Renderer::frame`, because a public mutable path to the Buffer would be a hole
  straight through the guarantee. Read-only Buffer access stays public — the
  diff and the tests need it.
- `Surface::sub` reborrows, so only one sub-surface can be live at a time. This
  is a real constraint on the API shape, and the reason geometry splitting stays
  on `Rect` rather than moving onto `Surface`.
