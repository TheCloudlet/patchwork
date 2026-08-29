# Patchwork — Domain Context

The vocabulary of this project. Glossary only — no implementation details, no
plans, no rationale. Rationale for hard-to-reverse choices lives in `docs/adr/`.

## Rect

An axis-aligned rectangle in cell coordinates: top-left corner `(x, y)` with
width `w` and height `h`. Pure geometry, with no coordinate system of its own —
whether a given `Rect` is absolute or relative is determined by the role it
plays (see **Viewport** and **Area**).

A `Rect` covers `x .. x + w` horizontally and `y .. y + h` vertically, both
half-open: the right and bottom edges are *not* included. A `Rect` with `w = 0`
or `h = 0` is empty and covers no cells.

## Viewport

The region a drawable is permitted to paint into, in **absolute** coordinates
(measured from the Buffer's top-left corner). A viewport is a property of a
**Surface**, not something a drawable is handed separately — a drawable never
sees its own viewport and cannot ask where it is.

A viewport is not a suggestion. Anything a drawable emits outside its viewport
is discarded — see **Clipping**.

## Area

The position and size a Pane or drawable *requests*, in coordinates **relative**
to the viewport it is handed. An area is a wish, not a fact: the effective
region is always the area intersected with the enclosing viewport, so an area
may be reduced or eliminated entirely.

Contrast with **Viewport**, which is absolute and already-negotiated. Both are
represented as a `Rect`; they are distinguished by name, not by type.

## Clipping

Discarding the parts of a drawable that fall outside its viewport. Clipping is
silent and lossless to everything else: a drawable that overflows loses its own
overflowing cells and never affects cells outside its viewport.

Clipping composes down the Pane tree by intersection, so a descendant can never
paint outside any of its ancestors' viewports.

## Surface

The painting handle a drawable is given: a Buffer restricted to one viewport.

A Surface addresses cells in coordinates **relative** to its own top-left
corner, and reports its own size — a drawable sees only its own space and
cannot observe or reach anything outside it. Writes that fall outside are
discarded, so **Clipping** is a property of the Surface rather than a rule each
drawable must remember to follow.

Narrowing a Surface to a sub-region intersects with the current viewport, never
widens it.

## Buffer

A grid of Cells, `rows` by `cols`, holding one frame's worth of screen content.
The root coordinate system: absolute coordinates are measured from its top-left
corner, `(0, 0)`.

## Cell

One terminal character position: a single character plus its Style.

## Pane

A node in the layout tree. A Pane has an **area**, its own content (drawables
painted as its background), and child Panes (painted on top, in order — later
children overwrite earlier ones).

## Drawable

Anything that can paint itself onto a Surface. In code, an implementor of the
`Draw` trait.

Drawables divide into three kinds that coexist deliberately:

- **Primitives** — Dot, Line, Rect, Text. Each carries its own coordinates,
  because those coordinates *are* the figure: a line without endpoints is not a
  line. This is the X11 drawing-primitive layer.
- **Decorations** — Border, Fill. These carry no coordinates at all. They take
  the shape of whatever Surface they are given, asking it for its size at draw
  time, so they stay correct when the space they occupy changes.
- **Composites** — Panes. These carry no figure of their own; they allocate
  space to other drawables and stack them. This is the window-hierarchy layer.

Space allocation belongs to the composite layer; a primitive's coordinates place
it *within* space already allocated; a decoration simply fills what it is given.

## Frame

One complete painting of the screen. The renderer keeps two buffers and emits
only the cells that differ between consecutive frames.

A frame always begins blank: whatever a frame does not paint is blank, never a
leftover from an earlier frame. A frame is therefore a pure function of state,
carrying nothing across from the frame before it.

Blanking costs a fill over already-allocated memory — the two buffers are
allocated once and reused for the program's lifetime. It costs no terminal
output, because unchanged cells are eliminated by the diff before anything is
written.

## Presenting

Ending a frame: diffing it against the previous frame and emitting the changed
cells to the terminal. The point at which drawing becomes visible.

## Scrolling

Showing a window onto content larger than the space available for it.

Scrolling is **not a mechanism here — it is a coordinate**. Content is drawn at
an offset, and whatever falls outside the Surface is clipped away by the same
rule that clips everything else. There is no scroll state on a Surface and no
scrolling drawable; scrolling one line means drawing the next frame with the
offset one larger.

This follows from every frame being repainted from blank (see **Frame**). A
scroll register — a persistent offset applied during addressing — is what
hardware needs when it *cannot* repaint, and does not apply.

## Immediate mode

Painting driven entirely by the caller — acquire a Surface, draw, present,
repeat — with nothing surviving between frames. The whole toolkit is immediate
mode; a frame is a function of state and nothing else.

The Pane tree does not contradict this. A Pane tree is built during a frame and
dropped at the end of it; it is a description of one frame, not a scene that is
kept and mutated. Nothing is ever "updated" in place — the next frame is
described afresh.

On the C side a Surface handle is valid solely for the frame it was obtained
in, and nothing crossing the boundary is owned or freed by the caller.
