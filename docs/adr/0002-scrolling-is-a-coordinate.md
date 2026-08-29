# Scrolling is a coordinate, not a mechanism

There is deliberately no scroll offset on `Surface`, no scrolling drawable, and
no scroll support anywhere in the toolkit. Scrolling is expressed by drawing
content at an offset and letting ordinary clipping discard what falls outside —
to scroll down one line, the next frame is drawn with the offset one larger.
This falls out of every frame being repainted from blank: once that is true,
scrolling is not an operation the framework needs to support, it is just a
different argument to the same drawing calls.

## Considered Options

- **A scroll offset on `Surface`.** This is what the NES PPU does with its
  scroll registers, and it was the obvious thing to reach for while reading
  about that hardware. Rejected on realising *why* the NES needs it: the PPU has
  no framebuffer and composites scanlines on the fly, so it physically cannot
  repaint. A scroll register is what you build when repainting is unavailable.
  We have a framebuffer and a diffing renderer, so the constraint that motivates
  the mechanism does not exist here.
- **A `ScrollView` drawable wrapping a child.** Rejected as unimplementable in
  this architecture: clipping happens inside `Surface::set`, so a wrapper cannot
  intercept a child's out-of-range writes. It would have to fabricate a Surface
  with an offset, which is just the first option wearing a different hat.

## Consequences

- Redrawing a long list means iterating only the visible slice — a caller-side
  concern, requiring nothing from the framework.
- If terminal-level scrolling (`\x1b[S` and friends) is ever wanted, that is an
  output optimisation belonging to the renderer, and still not a `Surface`
  concern.
